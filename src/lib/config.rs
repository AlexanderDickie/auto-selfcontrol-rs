use std::error::Error;
use std::{path::Path, process::Command, io::ErrorKind, fmt::Display};

use chrono::{self, NaiveTime, Local};
use serde::{Deserialize, Serialize};
use super::ResultE;
use std::fs;
use std::fmt;


#[derive(Deserialize, Serialize, Debug)]
pub struct Config {
    pub self_control_path: String,
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
    pub fn get_block_starts(&self) -> Vec<NaiveTime> {
        self.blocks
            .iter()
            .map(|block| block.0)
            .collect()
    }
    pub fn block_is_active(&self) -> Option<(NaiveTime, NaiveTime)> {
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
    pub fn remove_agent(&self, name: &str) -> ResultE<()> {
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
    pub fn install_agent(&self, name: &str, plist: &str) -> ResultE<()> {
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
