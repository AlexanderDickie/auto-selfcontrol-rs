use std::env;
use std::path::Path;
use std::error::Error;
use std::fs;
use clap::{
    ArgGroup,
    arg, 
    command,
};
use main_error::MainError;

mod lib;
use lib::config::{Config};

fn main() -> Result<(), MainError>{
    let matches = command!()
        .args(&[
            arg!(-d --deploy "Remove existing launch agents, parses config file, then install launch agents
                with respect to your config"),

            arg!(-e --execute "If we are in currently in an active block, activates SelfControlApp until \
                the block ends"),

            arg!(-r --remove_agents "Remove any launch agent installed by this program"),

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
                // require exactly one of the above flags
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
    // if path to config does not exist, create it 
    fs::create_dir_all(&config_dir)?;
    let config_path = config_dir.join("config.json");

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
        match lib::execute(&config) {
            Ok(_) => (), 
            Err(e) => println!("{:?}", e),
        }
    }
    if matches.get_flag("remove_agents") {
        lib::remove_all_agents(&config)?;
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

r#"
[paths]
selfcontrol = "/Applications/SelfControl.app/Contents/MacOS/org.eyebeam.SelfControl"   
launch_agents = "{}"

[blocks]

* = [
    (09:30 -> 13:50),
    (20:00 -> 08:30)
]

Wed = []

[Sat, Sun] = [
    (11:00 -> 12:15),
    (14:30 -> 18:36)
]
"# , {launch_agents_path}
        )
    )}

