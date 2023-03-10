use std::fmt::{self, Display};
use std::{fs, env};
use std::collections::HashMap;
use std::{process::Command, path::Path};
use std::io::{self, ErrorKind};
use std::error::Error;
use core_foundation::string::CFString;
use core_foundation::propertylist::{CFPropertyList, CFPropertyListSubClass};
use chrono::{self, Timelike, NaiveTime, Local, Duration};
use serde::{Deserialize, Serialize};


const TEMP_AGENT: &str = "com.temp-auto-selfcontrol-rs.plist";
const MAIN_AGENT: &str = "com.main-auto-selfcontrol-rs.plist";

type ResultE<T> = std::result::Result<T, Box<dyn Error>>;

pub fn deploy(config: &Config) -> ResultE<()> {
    /*
    install a launch agent which calls execute on this binary at the heads
    of each block
    */
    let command = env::current_exe()?;
    let command = command.to_str().ok_or_else(|| "invalid path to this binary")?;
    let args = vec!["--execute"];
    let block_starts = config.get_block_starts();
    let schedule = LaunchAgentSchedule::Calendar(&block_starts);

    let plist = build_launch_agent_plist(
        MAIN_AGENT,
        command,
        &args,
        &schedule,
        true,
    );
    config.install_agent(MAIN_AGENT, &plist)?;

    // we may be now be in an active block- so call execute on this binary
    let this_binary = std::env::current_exe()?;
    Command::new(&this_binary)
        .arg("--execute")
        .output()?;
    Ok(())
}

pub fn execute(config: &Config) -> ResultE<()> {
    /*
    check if we are currently within an active block, and if so activates
    SelfControl (depending on current state of SelfControl) until the end of 
    the block
    */
    let block = config.block_is_active();
    if block == None {
        // not within an active block
        return Ok(());
    }
    let (_, block_end) = block.unwrap();

    let now = Local::now().time();
    let time_to_block_end = duration_between(now, block_end);

    let sc_active = SC_is_active(&config.self_control_path)?;
    if sc_active == None {
        // sc is not active, start sc for duration of block
        insist_SC_begin_block(&config.self_control_path, block_end)?;
        return Ok(());
    }

    let sc_end = sc_active.unwrap();
    let time_to_sc_end = duration_between(now, sc_end);

    if time_to_sc_end >= time_to_block_end {
        // sc finishes after block ends, do nothing
        return Ok(());
    }

    /*
    self control finishes before this current block ends
    there is no sc-cli option to extend the block, so we need to install a launch agent
    to call execute on this binary when sc ends 
    */
    let command = env::current_exe()?;
    let command = command.to_str().ok_or_else(|| io::Error::new(ErrorKind::Other, "invalid path to binary"))?;
    let args = vec!["--execute"];
    // we have only minute precision to schedule the launch agent
    let calendar = vec![sc_end.with_second(0).unwrap() + chrono::Duration::minutes(1)];
    let schedule = LaunchAgentSchedule::Calendar(&calendar);
    let plist = build_launch_agent_plist(
        TEMP_AGENT,
        command,
        &args,
        &schedule,
        false,
    );
    config.install_agent(TEMP_AGENT, &plist)?;

    Ok(())
}

pub fn remove_agents(config: &Config) -> ResultE<()> {
    config.remove_agent(TEMP_AGENT)?;
    config.remove_agent(MAIN_AGENT)?;
    Ok(())
}

fn duration_between(start: NaiveTime, end: NaiveTime) -> Duration {
    let dif = end - start;
    match start < end {
        true => dif,
        false => Duration::hours(24) + dif, 
    }
} 

#[allow(non_snake_case)]
fn insist_SC_begin_block(path: &str, end: NaiveTime) -> ResultE<()> {
    /*
    self control requires the user to input their password to install a helper tool,
    if the user refuses, start sc again (the helper prompt will immediately  reappear)
    */
    loop {
        let now = Local::now().time();
        let duration = duration_between(now, end);

        match SC_begin_block(path, duration) {
            Ok(_) => return Ok(()),

            Err(e) => match e {
                SelfControlError::UserCancelledHelper => continue,
                _ => return Err(e.into()),

            }
        }
    }

}

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Deserialize, Serialize, Debug)]
pub struct Config {
    self_control_path: String,
    launch_agents_path: String,
    blocks: Vec<(NaiveTime, NaiveTime)>,
}

impl Config {
    pub fn build(config_path: &Path) -> ResultE<Self> {
        use ConfigError::*;
        let config_file = fs::read_to_string(config_path)?;
        let mut config: Config = serde_json::from_str(&config_file)?;
        /*
        Check sequence of blocks are valid
        */

        // exit on 0 blocks, 1 block will have valid logic nomatter
        let blocks = &mut config.blocks;
        if blocks.len() == 0 {
            return Err(NoBlocks.into());
        } else if blocks.len() == 1 {
            return Ok(config);
        }

        blocks.sort_by(|a, b| (&a.0).partial_cmp(&b.0).unwrap());
        // no pairs of blocks should overlap
        // if the last block crosses midnight, check if it overlaps with first block
        let first_block = blocks.first().unwrap();
        let last_block = blocks.last().unwrap();
        if last_block.1 < last_block.0 && last_block.1 >= first_block.0 {
            return Err(OverlappingBlocks.into());
        }
        let overlapping = blocks.windows(2).filter(|pair| pair[0].1 >= pair[1].0);
        if overlapping.count() > 0 {
            return Err(OverlappingBlocks.into());
        }

        Ok(config)
    }
    fn get_block_starts(&self) -> Vec<NaiveTime> {
        self.blocks
            .iter()
            .map(|block| block.0)
            .collect()
    }
    fn block_is_active(&self) -> Option<(NaiveTime, NaiveTime)> {
        let now = Local::now().time();
        self.blocks
            .iter()
            .filter(|block| {
                let (start, end) = block;
                if start < end {
                    now >= *start && now < *end
                } else { 
                    now >= *start || now < *end
                }
            })
            .next()
            .copied()
    }
    fn remove_agent(&self, name: &str) -> ResultE<()> {
        Command::new("launchctl")
            .arg("remove")
            .arg(&name)
            .output()?;

        match fs::remove_file(format!("{}/{}", self.launch_agents_path, name)) {
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
    // todo: if a temp agent tries to install another temp agent it will fail
    fn install_agent(&self, name: &str, plist: &str) -> ResultE<()> {
        self.remove_agent(name)?;
        /*
        writes plist to launch agents folder then loads it
        if plist with same name exits, its overwrites it
        */
        let path = Path::new(&self.launch_agents_path).join(name);
        fs::write(&path, plist)?;

        Command::new("launchctl")
            .arg("load")
            .arg(&path)
            .output()?;
        Ok(())
    }
}


////////////////////////////////////////////////////////////////////////////////////////////////////


fn build_plist_header(name: &str) -> String {
    format!(
r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{}</string>"#
, name)
}

fn build_plist_commands(command: &str, args: &Vec<&str>) -> String {
    let args = args
        .iter()
        .map(|arg| format!(
r#"        <string>{}</string>"#
            , arg))
        .collect::<Vec<String>>()
        .join("\n");

    format!(
r#"    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
{}
    </array>"#
,command, args)
}

enum LaunchAgentSchedule<'a> {
    Calendar(&'a Vec<NaiveTime>), 
    Periodic(Duration), 
}
fn build_plist_schedule(schedule: &LaunchAgentSchedule, run_at_load: bool) -> String {
    let timings = match schedule {
        LaunchAgentSchedule::Periodic(duration) => {
        format!(
r#"    <key>StartInterval</key>
        <integer>{}<\integer>"#
        ,duration.num_seconds())
        },

        LaunchAgentSchedule::Calendar(start_times) => {
            let start_times = start_times
                .iter()
                .map(|time| format!(
r#"    <dict>
        <key>Minute</key>
        <integer>{}</integer>
        <key>Hour</key>
        <integer>{}</integer>
    </dict>"#, 
                time.minute(), time.hour()))
                .collect::<Vec<String>>()
                .join("\n");


"    <key>StartCalendarInterval</key>\n".to_string() + &start_times
        }
    };

    // if agent is scheduled to execute when computer is shut down, it will not when next online- run at load 
    // to mitigate this
    if run_at_load {
        return timings +
&r#"
    <key>RunAtLoad</key>
    <true/>"#.to_string();
    } else {
    return timings;
    }
}

fn build_plist_footer() -> String {
r#"</dict>
</plist>"#.to_string()
}

fn build_launch_agent_plist(
    name: &str,
    command: &str,
    args: &Vec<&str>,
    schedule: &LaunchAgentSchedule,
    run_at_load: bool)
    -> String 
{
    let parts = vec![
        build_plist_header(name),
        build_plist_commands(command, args),
        build_plist_schedule(schedule, run_at_load),
        build_plist_footer(),
    ];
    parts.join("\n")
}

#[link(name = "CoreFoundation", kind = "framework")]
extern {
    fn CFPreferencesSetAppValue(key: CFString, value: CFPropertyList, applicationID: CFString);
    fn CFPreferencesAppSynchronize( applicationID: CFString) -> bool;
}


#[allow(non_snake_case)]
fn SC_begin_block(SC_path: &str, duration: chrono::Duration) -> Result<(), SelfControlError> {
    use SelfControlError::*;
    // set block duration of selfcontrol
    let mins = (duration.num_seconds() as f64 / 60.0).ceil() as u32;
    unsafe {
        CFPreferencesSetAppValue(
            CFString::new("BlockDuration"),
            CFString::new(&mins.to_string()).to_CFPropertyList(),
            CFString::new("org.eyebeam.SelfControl")
        );
        CFPreferencesAppSynchronize(CFString::new("org.eyebeam.SelfControl"));
    }

    // start self control
    let output = Command::new(SC_path)
        .arg("start")
        .output()
        .map_err(|e| SelfControlError::CommandError(e))?;
    let stderr = String::from_utf8(output.stderr)
        .map_err(|e| SelfControlError::ParseError(e))?;
    // check if user refused helper installation
    if stderr.contains("Authorization cancelled") {
        return Err(UserCancelledHelper.into());
    }
    // check for other non success message
    if !stderr.contains(&"INFO: Block successfully added.") {
        return Err(NoSuccessMsg.into());
    } else {
        Ok(())
    }
}

#[allow(non_snake_case)]
fn SC_parse_print_settings(stderr: &str) -> ResultE<HashMap<String, String>> {
    /* 
    parse the settings dictionary from the output of calling print-settings on selfcontrol
    */
    let mut settings = stderr
        .trim_start_matches(|c| c != '{')
        .trim_end_matches(|c| c != '}')
        .replace(" ", "")
        .replace("\n", "");
    // remove { and }, and remove final ;
    settings.remove(0);
    settings.pop();
    settings.pop();

    let mut settings_map: HashMap<String, String> = HashMap::new();
    for line in settings.split(";") {
        let mut pairs = line.split("=");
        let key = pairs
            .next()
            .ok_or(SelfControlCliError::InvalidPlistFormat)?
            .to_owned();
        let value = pairs
            .next()
            .ok_or(SelfControlCliError::InvalidPlistFormat)?
            .to_owned();
        settings_map.insert(key, value);
    }
    Ok(settings_map)
}

#[allow(non_snake_case)]
fn SC_is_active(SC_path: &str) -> ResultE<Option<NaiveTime>> {
    /*
    checks if self control is currently active, if so returns the time it will end
    */
    let output = Command::new(SC_path)
        .arg("print-settings")
        .output()?;
    let stderr = String::from_utf8(output.stderr)?;
    let settings_map =  SC_parse_print_settings(&stderr)?;

    let is_active = settings_map
        .get("BlockIsRunning")
        .ok_or(SelfControlCliError::MisingPlistKey)?;

    // self control is not active
    if is_active == "0" {
        return Ok(None);
    }

    // self control is active 
    let end_date = settings_map
        .get("BlockEndDate")
        .ok_or(SelfControlCliError::MisingPlistKey)?;

    // our date value has weird format- "\"2022-12-3022:25:27+0000\"" so format it
    let end_date = end_date.replace("\"", "");
    let end_date = &end_date[10..end_date.chars().count()-5];
    let end_date = NaiveTime::parse_from_str(end_date, "%H:%M:%S")?;
    Ok(Some(end_date))
}


// Errors 
////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
enum ConfigError {
    NoBlocks,
    OverlappingBlocks,
}
impl Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ConfigError::NoBlocks => write!(f, "no blocks specified in config"),
            ConfigError::OverlappingBlocks => write!(f, "overlapping blocks specified in config"),
        }
    }
}
impl Error for ConfigError {}


#[derive(Debug)]
enum SelfControlError {
    UserCancelledHelper,
    NoSuccessMsg,
    CommandError(std::io::Error),
    ParseError(std::string::FromUtf8Error)
}
impl Display for SelfControlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SelfControlError::UserCancelledHelper => write!(f, "User cancelled helper authorization"),
            SelfControlError::NoSuccessMsg => write!(f, "no success message in SelfControl stderr output"),
            SelfControlError::CommandError(e) => write!(f, "error executing self control binary: {}", e),
            SelfControlError::ParseError(e) => write!(f, "error parsing self control output: {}", e),
        }
    }
}
impl Error for SelfControlError {}


#[derive(Debug)]
enum SelfControlCliError {
    InvalidPlistFormat,
    MisingPlistKey
}
impl Display for SelfControlCliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SelfControlCliError::MisingPlistKey => write!(f, "missing key in SelfControl cli plist"),
            SelfControlCliError::InvalidPlistFormat => write!(f, "invalid SelfControl cli plist"),
        }
    }
}
impl Error for SelfControlCliError {}


////////////////////////////////////////////////////////////////////////////////////////////////


/*
many of these tests need to be manually inspected by looking at the consequent behavior of the
self control program
*/
#[cfg(test)]
mod tests {
    use super::*;
    const SC_PATH: &str = "/Applications/SelfControl.app/Contents/MacOS/org.eyebeam.SelfControl";

    #[test]
    fn build_config() {
        let config = 
r#"{
    "self_control_path": "/Applications/SelfControl.app/Contents/MacOS/org.eyebeam.SelfControl",
    "launch_agents_path": "/Users/username/Library/LaunchAgents",
    "blocks": [
        ["12:00:00",
         "13:00:00"],

        ["14:00:00",
         "15:00:00"]
    ]
}
"#;
        fs::write("test_config.json", config).unwrap();
        let path = Path::new("test_config.json");
        let config = Config::build(&path.to_path_buf());
        println!("{:?}", config);
        fs::remove_file("test_config.json").unwrap();
    }

    #[test]
    fn sc_parse_cli_settings_generic() {
        let s = "{
            k1=v1;
            k2=v2;
            }".to_string();
        let output = Command::new(SC_PATH)
            .arg("print-settings")
            .output().unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();
        let settings =  SC_parse_print_settings(&stderr);
        println!("{:?}", settings);
    }

    #[test]
    fn sc_is_active_generic() {
        let o = SC_is_active(SC_PATH);
        println!("{:?}", o);
    }

    #[test]
    fn SC_begin_block_generic() {
        let now = Local::now().time();
        SC_begin_block(SC_PATH, Duration::minutes(2)).unwrap();
    }

    #[test]
    fn ffi_generic() {
        let name = CFString::new("org.eyebeam.SelfControl");
        unsafe {
            println!("cfprefs {}", CFPreferencesAppSynchronize(name));
        }
    }

    #[test]
    fn build_plist_commands_generic() {
        let command: &str = "cmd".into();
        let args = vec!["arg1", "arg2"];

        let output = build_plist_commands(command, &args);
        let expected = 
r#"    <key>ProgramArguments</key>
    <array>
        <string>cmd</string>
        <string>arg1</string>
        <string>arg2</string>
    </array>"#.to_string();                             
        for pair in output.lines().zip(expected.lines()) {
            assert_eq!(pair.0, pair.1);
        }
    }


    #[test]
    fn build_launch_agent_periodic() {
        let command: &str = "cmd".into();
        let args = vec!["arg1".to_string(), "arg2".to_string()];
        let start_date = Local::now().naive_local();
        let schedle = LaunchAgentSchedule::Periodic(Duration::seconds(60 * 5));

        let output = build_launch_agent_plist(
            "name".into(),
            "touch".into(),
            &vec!["arg1".into(), "arg2".into()],
            &schedle,
        );
        println!("{}", output); 
    }

    #[test]
    fn build_launch_agent_calendar() {
        let command: &str = "cmd".into();
        let args = vec!["arg1".to_string(), "arg2".to_string()];
        let start_date = Local::now().naive_local();
        let times = vec![
            NaiveTime::from_hms(15, 10, 0),
            NaiveTime::from_hms(10, 55, 20)];
        let schedle = LaunchAgentSchedule::Calendar(&times);

        let output = build_launch_agent_plist(
            "name".into(),
            "touch".into(),
            &vec!["arg1".into(), "arg2".into()],
            &schedle,
        );
        println!("{}", output); 
    }

    #[test]
    fn persevere_SC_begin_block_generic() {
        let now = Local::now().time();
        let end = now + chrono::Duration::minutes(2);
        insist_SC_begin_block(SC_PATH, end).unwrap();
    }
}
