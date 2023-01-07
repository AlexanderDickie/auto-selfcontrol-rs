use std::env;
use std::path::PathBuf;
use std::error::Error;
use std::fs;

mod lib;
use lib::{
    Config,
    run,
    InvalidProgramArgument,
};

fn main() -> Result<(), Box<dyn Error>>{

    let home_dir = env::var_os("HOME")
        .ok_or_else(|| "HOME environment variable not set")?;
    let home_dir = PathBuf::from(home_dir.to_str().unwrap());

    let config_dir = home_dir 
        .join(".config")
        .join("auto-self-control-rs/");
    fs::create_dir_all(&config_dir)?;

    // write the example config file if it doesn't exist
    let config_path = config_dir.join("config.json");
    if !config_path.exists() {
        let example_config = build_example_config(home_dir);
        fs::write(&config_path, example_config)?;
    }

    let args = env::args().collect::<Vec<String>>();
    if args.len() != 2 {
        println!("{}", InvalidProgramArgument);
        return Err(InvalidProgramArgument.into()); 

    }

    let config = Config::build(&config_path)?;
    run(&config, &args[1])?;

    Ok(())
}

fn build_example_config(home_dir: PathBuf) -> String {
    let launch_agents_path = home_dir.join("Library/LaunchAgents/");
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
}}"#, {launch_agents_path.to_str().unwrap()})
}

