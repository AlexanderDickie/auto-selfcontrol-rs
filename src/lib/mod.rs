use chrono::{self, Duration, Local};
use std::env;

pub mod config;
mod plist;
mod begin_block;

mod utils;
pub use utils::ResultE;

use begin_block::begin_block_until;
use config::Config;
use plist::LaunchAgentSchedule;

const MAIN_AGENT: &str = "com.main-auto-selfcontrol-rs.plist";

pub fn deploy(config: &Config) -> ResultE<()> {
    let command = env::current_exe()?;
    let command = command
        .to_str()
        .ok_or_else(|| "invalid path to this binary")?;

    let plist = plist::build_launch_agent_plist(
        MAIN_AGENT,
        command,
        &vec!["--execute"],
        &LaunchAgentSchedule::Periodic(Duration::seconds(30)),
        true,
    );
    config.install_agent(MAIN_AGENT, &plist)?;
    Ok(())
}

pub fn execute_for_duration(config: &Config, duration: Duration) -> ResultE<()> {
    let now = Local::now().naive_local();
    begin_block_until(config, now + duration)
}

pub fn execute(config: &Config) -> ResultE<()> {
    let block = config.get_currently_active_block();
    let Some((block_start_time, block_end_time)) = block else { return Ok(()) };

    let mut block_end = Local::now().naive_local().date().and_time(block_end_time);
    if block_start_time > block_end_time {
        block_end += Duration::days(1);
    }

    begin_block_until(config, block_end)
}

