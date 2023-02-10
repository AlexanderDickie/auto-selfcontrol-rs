use std::env;
use std::path::Path;
use std::error::Error;
use std::fs;
use clap::{
    ArgGroup,
    arg, 
    command,
};

mod lib;
use lib::Config;

fn main() -> Result<(), Box<dyn Error>>{

    // parse command line flags
    let matches = command!()
        .args(&[
            arg!(-d --deploy "Remove existing launch agents, parses config file, then install launch agents
                with respect to your config"),

            arg!(-e --execute "If we are in currently in an active block, activates SelfControlApp until \
                the block ends"),

            arg!(--remove_agents "Remove any launch agent installed by this program"),

            arg!(--write_example_config "Writes an example configuration file to \
                ~/.config/auto-self-control-rs/config.json"),
        ])
        .group(
            ArgGroup::new("commands")
                .args([
                    "deploy",
                    "execute",
                    "remove_agents",
                    "write_example_config",
                ])
                // require one and only one of the above flags
                .multiple(false)
                .required(true)
        )
        .get_matches();

    // build path to config file
    let home_dir = env::var_os("HOME")
        .ok_or_else(|| "HOME environment variable not set")?;
    let home_dir = Path::new(&home_dir);
    let config_dir = home_dir 
        .join(".config")
        .join("auto-self-control-rs/");
    // if path does not exist, create it 
    fs::create_dir_all(&config_dir)?;
    let config_path = config_dir.join("config.json");

    // write example config file
    if matches.get_flag("write_example_config") {
        let example_config = build_example_config(home_dir)?;
        fs::write(&config_path, example_config)?;
        return Ok(());
    }

    let config = Config::build(&config_path)?;

    if matches.get_flag("deploy") {
        lib::deploy(&config)?;
    }

    if matches.get_flag("execute") {
        lib::execute(&config)?;
    }

    // remove_agents
    if matches.get_flag("remove_agents") {
        lib::remove_agents(&config)?;
    }

    Ok(())
}

fn build_example_config(home_dir: &Path) -> Result<String, Box<dyn Error>> {
    let launch_agents_path = home_dir
        .join("Library/LaunchAgents/");
    let launch_agents_path = launch_agents_path
        .to_str()
        .ok_or("Could not convert launch agent path to string")?;
    Ok(
        format!(
r#"{{
    "self_control_path": "/Applications/SelfControl.app/Contents/MacOS/org.eyebeam.SelfControl",   
    "launch_agents_path": "{}",                          
    "blocks": [
        [ 
        "10:20:00",
        "12:00:00"
        ],

        [
        "17:55:00",
        "2:00:00"
        ]
    ]
}}"#, {launch_agents_path})
            )
}

