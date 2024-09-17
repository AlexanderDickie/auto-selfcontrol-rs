use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
    process::Command,
    env
};
use super::ResultE;
use chrono::{self, Datelike, Local, NaiveTime};
use serde::Deserialize;
use std::{collections::HashMap, fs};
use security_framework::passwords::{get_generic_password, set_generic_password};
use serde::de::{self, Visitor};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub paths: Paths,
    pub auto_password_input: AutoPasswordInput,
    #[serde(deserialize_with = "deserialize_blocks")]
    blocks: HashMap<Day, Vec<(NaiveTime, NaiveTime)>>,
}

#[derive(Deserialize, Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub enum Day {
    All,
    #[serde(untagged)]
    WeekDay(chrono::Weekday),
}

#[derive(Debug, Deserialize)]
#[serde(default, rename_all = "kebab-case", deny_unknown_fields)]
pub struct Paths {
    pub self_control: PathBuf,
    launch_agents: PathBuf,
}

impl Default for Paths {
    fn default() -> Self {
        let launch_agents = env::var_os("HOME")
            .map(|path| Path::new(&path).join("Library/LaunchAgents/"))
            .expect("HOME environment variable not set");

        Self {
            self_control: "/Applications/SelfControl.app/Contents/MacOS/org.eyebeam.SelfControl".into(),
            launch_agents,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default, rename_all = "kebab-case", deny_unknown_fields)]
pub struct AutoPasswordInput {
    enable: bool,
    account_name: String,
}

impl Default for AutoPasswordInput {
    fn default() -> Self {
        Self {
            enable: false,
            account_name: get_account_name(),
        }
    }
}

fn get_account_name() -> String {
    env::var("USERNAME")
        .or_else(|_| env::var("USER"))
        .unwrap_or_else(|_| {
            let output = Command::new("whoami").output().expect("failed to get username from env-vars, set this manually in config");
            String::from_utf8(output.stdout).expect("failed to get username from env-vars, set this manually in config")
        })
        .trim()
        .to_string()
}

impl AutoPasswordInput {
    const ITEM_NAME: &'static str = "auto_selfcontrol_rs";

    pub fn get_pswd(&self) -> ResultE<Option<String>> {
        if self.enable {
            get_generic_password(Self::ITEM_NAME, &self.account_name)
                .map(|bytes| String::from_utf8(bytes).ok())
                .map_err(|e| e.into())
        } else {
            Ok(None)
        }
    }

    pub fn set_pswd(&self, pswd: &str) -> ResultE<()> {
        if self.enable {
            set_generic_password(Self::ITEM_NAME, &self.account_name, pswd.as_bytes())
                .map_err(|e| e.into())
        } else {
            Ok(())
        }
    }
}

fn deserialize_blocks<'de, D>(
    deserializer: D,
) -> Result<HashMap<Day, Vec<(NaiveTime, NaiveTime)>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct DaysTimesPair {
        days: Vec<Day>,
        #[serde(deserialize_with = "deserialize_times")]
        times: Vec<(NaiveTime, NaiveTime)>,
    }

    fn deserialize_times<'de, D>(deserializer: D) -> Result<Vec<(NaiveTime, NaiveTime)>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct NaivesVisitor;

        impl<'de> Visitor<'de> for NaivesVisitor {
            type Value = Vec<(NaiveTime, NaiveTime)>;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a sequence of (xx::xx, xx::xx)")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut times_vec = vec![];
                while let Some(pair) = seq.next_element::<Vec<&str>>()? {
                    if pair.len() != 2 {
                        return Err(de::Error::custom("More than two times specified"));
                    }
                    let (start, end) = (
                        NaiveTime::parse_from_str(pair[0], "%H:%M"),
                        NaiveTime::parse_from_str(pair[1], "%H:%M"),
                    );

                    if start.is_err() || end.is_err() {
                        return Err(de::Error::custom("invalid time format"));
                    }
                    times_vec.push((start.unwrap(), end.unwrap()));
                }
                return Ok(times_vec);
            }
        }
        deserializer.deserialize_seq(NaivesVisitor)
    }

    struct BlocksVisitor;
    impl<'de> Visitor<'de> for BlocksVisitor {
        type Value = HashMap<Day, Vec<(NaiveTime, NaiveTime)>>;
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a sequence of pairs of days and times")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            let mut day_times_map: HashMap<Day, Vec<(NaiveTime, NaiveTime)>> = HashMap::new();
            while let Some(v) = seq.next_element::<DaysTimesPair>()? {
                for day in v.days {
                    if day_times_map.contains_key(&day) {
                        return Err(serde::de::Error::custom("duplicate day in config"));
                    }
                    day_times_map.insert(day, v.times.clone());
                }
            }
            Ok(day_times_map)
        }
    }

    deserializer.deserialize_seq(BlocksVisitor)
}

impl Config {
    pub fn build(config_path: &Path) -> ResultE<Self> {
        let config_file = fs::read_to_string(config_path)?;
        serde_yaml::from_str(&config_file).map_err(|e| e.into())
    }

    pub fn get_currently_active_block(&self) -> Option<(NaiveTime, NaiveTime)> {
        let now = Local::now();
        let time_now = Local::now().time();
        let weekday = now.date_naive().weekday();

        let blocks: &Vec<(NaiveTime, NaiveTime)>;
        if let Some(v) = self.blocks.get(&Day::WeekDay(weekday)) {
            blocks = v;
        } else if let Some(v) = self.blocks.get(&Day::All) {
            blocks = v;
        } else {
            return None;
        }
        Self::find_block(time_now, blocks)
    }

    fn find_block(
        now: NaiveTime,
        blocks: &Vec<(NaiveTime, NaiveTime)>,
    ) -> Option<(NaiveTime, NaiveTime)> {
        blocks
            .iter()
            .filter(|(start, end)| {
                if start < end {
                    now >= *start && now < *end
                } else {
                    now >= *start || now < *end
                }
            })
            .next()
            .copied()
    }

    pub fn remove_agent(&self, name: &str) -> ResultE<()> {
        Command::new("launchctl")
            .arg("remove")
            .arg(&name)
            .output()?;

        let path = self.paths.launch_agents.join(name);
        match fs::remove_file(path) {
            Ok(_) => Ok(()),
            Err(e) => {
                if e.kind() == ErrorKind::NotFound {
                    Ok(())
                } else {
                    Err(Box::new(e))
                }
            }
        }
    }

    pub fn install_agent(&self, name: &str, plist: &str) -> ResultE<()> {
        self.remove_agent(name)?;
        let path = Path::new(&self.paths.launch_agents).join(name);
        fs::write(&path, plist)?;

        Command::new("launchctl").arg("load").arg(&path).output()?;
        Ok(())
    }
}

pub fn build_example_config() -> String {
    format!(
"auto-password-input:
  #enable: ...  # optional, defaults to false 
  #account_name: ... # optional, defaults to $USER else $whoami 

blocks:
- days: [Mon, Wed]
  times: [[11:00, 13:00], [17:00, 19:30]]
- days: [Thu]
  times: [[21:00, 08:00]] # This will block from Thursday 21:00 until Friday 08:00 
- days: [All]
  times: [[8:00, 9:00]] 
# Explicitly defined weekdays override 'All'- eg Monday will not contain the 8:00 -> 9:00 block 

paths:
    #self-control: ... # optional, defaults to /Applications/SelfControl.app/Contents/MacOS/org.eyebeam.SelfControl
    #launch-agents: ... # optional, defaults to ~/Library/LaunchAgents/"
    )
}
