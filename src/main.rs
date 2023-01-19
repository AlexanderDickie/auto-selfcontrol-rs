use std::env;
use std::path::Path;
use std::error::Error;
use std::fs;

mod lib;
use lib::{
    Config,
    run,
    InvalidProgramArgument,
};

fn main() -> Result<(), Box<dyn Error>>{

    // find path to config file
    let home_dir = env::var_os("HOME")
        .ok_or_else(|| "HOME environment variable not set")?;
    let home_dir = Path::new(&home_dir);
    let config_dir = home_dir 
        .join(".config")
        .join("auto-self-control-rs/");
    // if dir does not exist, create it 
    fs::create_dir_all(&config_dir)?;

    // write the example config file if the config file does not exist 
    let config_path = config_dir.join("config.json");
    if !config_path.exists() {
        let example_config = build_example_config(home_dir);
        fs::write(&config_path, example_config)?;
    }

    // parse command line argument, displaying usage on error
    let args = env::args().collect::<Vec<String>>();
    if args.len() != 2 {
        println!("{}", InvalidProgramArgument);
        return Err(InvalidProgramArgument.into()); 
    }

    // execute logic
    let config = Config::build(&config_path)?;
    run(&config, &args[1])?;
    Ok(())
}

fn build_example_config(home_dir: &Path) -> String {
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

