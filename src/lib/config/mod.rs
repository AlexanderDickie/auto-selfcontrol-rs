use std::collections::HashSet;
use std::error::Error;
use std::fmt::{Debug, Formatter};
use std::{path::{Path, PathBuf}, process::Command, io::ErrorKind, fmt::Display};

use chrono::{self, NaiveTime, Local, Weekday};
use super::ResultE;
use std::{fs, fmt, collections::HashMap};
use chrono::Datelike;

mod parse;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
enum Day {
    Default,
    Weekday(Weekday),
}
type ParsedBlocks = Vec<(Vec<Day>, Vec<(NaiveTime, NaiveTime)>)>;
type ParsedPaths = (String, String);

use chumsky::prelude::*;
pub struct Config {
    pub selfcontrol_path: PathBuf,
    launch_agents_path: PathBuf,
    blocks_map: HashMap<Day, Vec<(NaiveTime, NaiveTime)>>,
}

impl Config {
    pub fn build(config_path: &Path) -> ResultE<Self> {
        let config_file = fs::read_to_string(config_path)?;
        let (paths, blocks): (ParsedPaths, ParsedBlocks) = parse::parse_config()
            .parse(config_file)
            .map_err(|err| ConfigError::ParseError(err.into()))?;
        // check each present weekday has at most one block sequence defined for it
        let duplicate_days = blocks
            .iter()
            .fold((HashSet::<&Day>::new(), HashSet::new()), |mut acc, (days, _)| {
                let (seen, duplicates) = &mut acc;
                for day in days {
                    if seen.contains(&day) {
                        duplicates.insert(*day);
                    }
                    seen.insert(&day);
                }
                acc
            }).1.into_iter().collect::<Vec<_>>();
        if duplicate_days.len() > 0 {
            return Err(ConfigError::ManyDayDefinitions(duplicate_days.clone()).into());
        }
        // validate each block sequence present
        for (days, block_seq) in blocks.iter() {
            Self::validate_block_sequence(days, block_seq)?;
        }
        // build hashmap of Day -> block sequence
        let blocks_map = blocks
            .into_iter()
            .fold(HashMap::new(), |mut acc, (days, blocks)| {
                for day in days {
                    acc.insert(day, blocks.clone());
                }
                acc
            });
        // build paths 
        let selfcontrol_path = PathBuf::from(paths.0);
        let launch_agents_path = PathBuf::from(paths.1);
        Self::validate_path(&selfcontrol_path)?;
        Self::validate_path(&launch_agents_path)?;

        Ok(
            Config {
                selfcontrol_path,
                launch_agents_path,
                blocks_map,
            }
        )
    }
    fn validate_path(path: &PathBuf) -> ResultE<()> {
        match path.try_exists() {
            Err(_) => Ok(()),
            Ok(bool) => match bool {
                true => Ok(()),
                false => Err(ConfigError::InvalidPath(path.clone()).into())
            },
        }

    }
    fn validate_block_sequence(days: &Vec<Day>, block_seq: &Vec<(NaiveTime, NaiveTime)>) -> ResultE<()> {
        if block_seq.len() == 1 || block_seq.len() == 0{
            return Ok(());
        }
        // if the latest block crosses midnight, check if it overlaps with earliest block
        let mut blocks = block_seq.clone();
        blocks.sort_by(|a, b| (&a.0).partial_cmp(&b.0).unwrap());
        let first_block = blocks.first().unwrap();
        let last_block = blocks.last().unwrap();
        if last_block.1 < last_block.0 && last_block.1 >= first_block.0 {
            return Err(ConfigError::OverlappingBlocks(days.clone()).into());
        }
        // check no other blocks overlap
        let overlapping = blocks.windows(2).filter(|pair| pair[0].1 >= pair[1].0);
        if overlapping.count() > 0 {
            return Err(ConfigError::OverlappingBlocks(days.clone()).into());
        }
        Ok(())
    }
    pub fn get_all_block_start_times(&self) -> Vec<NaiveTime> {
        self.blocks_map
            .iter()
            .fold(vec![], |mut acc, (_, blocks)| {
                for (start, _) in blocks {
                    acc.push(start.clone());
                }
                acc
            })
    }
    fn find_block(now: NaiveTime, blocks: &Vec<(NaiveTime, NaiveTime)>) -> Option<(NaiveTime, NaiveTime)> {
        blocks
            .iter()
            .filter(|(start, end)| {
                if start < end {
                    now >= *start && now < *end
                } else {
                    now >= *start || now < *end
                }
            }).next()
            .copied()
    }
    pub fn get_active_block(&self) -> Option<(NaiveTime, NaiveTime)> {
        let now = Local::now();
        let time_now = now.time();

        let weekday = Day::Weekday(now.date_naive().weekday());
        let _default = Day::Default; 

        let blocks: &Vec<(NaiveTime, NaiveTime)>;
        if let Some(v) = self.blocks_map.get(&weekday) {
            blocks = v;
        } else if let Some(v) = self.blocks_map.get(&_default) {
            blocks = v;
        } else {
            return None;
        }
        Self::find_block(time_now, blocks)
    }
    pub fn remove_agent(&self, name: &str) -> ResultE<()> {
        Command::new("launchctl")
            .arg("remove")
            .arg(&name)
            .output()?;

        let path = self.launch_agents_path.join(name);
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
    // todo: if a temp agent tries to install another temp agent it will fail
    pub fn install_agent(&self, name: &str, plist: &str) -> ResultE<()> {
        self.remove_agent(name)?;
        /*
        writes plist to launch agents folder then loads it
        if plist with same name exits, overwrites it
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

#[derive(Debug)]
enum ConfigError {
    ParseError(ParseError),
    OverlappingBlocks(Vec<Day>),
    ManyDayDefinitions(Vec<Day>),
    InvalidPath(PathBuf),
}
#[derive(Debug)]
struct ParseError (String);
impl From<Vec<Simple<char>>> for ParseError {
    fn from(errs: Vec<Simple<char>>) -> Self {
        let err = &errs[0];
        let msg = if let Some(label) = err.label() {
            format!("Error when parsing {}: {}", label, err)
        } else {
            format!("Error when parsing: {}", err)
        };
        ParseError(msg)
    }
}
impl Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ConfigError::ParseError(ParseError(msg)) => write!(f, "{}", msg),
            ConfigError::OverlappingBlocks(days) => write!(f, "overlapping blocks specified in \
                config for days {:?}", days),
            ConfigError::ManyDayDefinitions(days) => write!(f, "There are multiple blocks definitions \
                for the days {:?}", days),
            ConfigError::InvalidPath(path) => write!(f, "The path {:?} is invaid", path),
        }
    }
}
impl Error for ConfigError {}
