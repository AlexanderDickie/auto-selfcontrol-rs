use std::env;
use std::process::Command;
use std::io::{self, ErrorKind};
use std::error::Error;
use chrono::{self, Timelike, Local};

pub mod config;
mod control;
mod plist;
mod utils;

use config::Config;
use plist::LaunchAgentSchedule;


const TEMP_AGENT: &str = "com.temp-auto-selfcontrol-rs.plist";
const MAIN_AGENT: &str = "com.main-auto-selfcontrol-rs.plist";

pub type ResultE<T> = std::result::Result<T, Box<dyn Error>>;

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

    let plist = plist::build_launch_agent_plist(
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
    let time_to_block_end = utils::duration_between(now, block_end);

    let sc_active = control::selfcontrol_is_active(&config.self_control_path)?;
    if sc_active == None {
        // sc is not active, start sc for duration of block
        control::selfcontrol_insist_begin_block(&config.self_control_path, block_end)?;
        return Ok(());
    }

    let sc_end = sc_active.unwrap();
    let time_to_sc_end = utils::duration_between(now, sc_end);

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
    let plist = plist::build_launch_agent_plist(
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

