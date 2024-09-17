use chrono::Duration;
use clap::{arg, command, Arg, ArgGroup};
use main_error::MainError;
use std::{env, fs, path::Path};
use rpassword;

mod lib;
use lib::config::{Config, self};

fn main() -> Result<(), MainError> {
    let matches = command!()
        .args(&[
            arg!(-d --deploy "Remove existing auto-self-control launch agent, parses config file, then install launch agents
                with respect to your config"),

            arg!(-e --execute "If we are in currently in an active block, activates SelfControlApp until \
                the block ends"),

            arg!(-w --write_example_config "Writes an example configuration file to \
                ~/.config/auto-selfcontrol-rs/config.aoml"),

            arg!(-p --set_keychain_password "Store the current MacOs user's password in keychain, which can then be used to automaticaly input into the SelfControl helper."),

            Arg::new("mins")
                .help("Start selfcontrol for a specified number of minutes")
                .short('s')
                .long("start_self_control")
                .num_args(1)
                .value_parser(|mins: &str| mins.parse::<usize>()),
        ])
        .group(
            ArgGroup::new("commands")
                .args([
                    "deploy",
                    "execute",
                    "write_example_config",
                    "mins",
                    "set_keychain_password"
                ])
                .multiple(false)
                .required(true)
        )
        .get_matches();

    let home_dir = env::var_os("HOME").ok_or_else(|| "HOME environment variable not set")?;
    let home_dir = Path::new(&home_dir);
    let config_dir = home_dir.join(".config").join("auto-selfcontrol-rs/");

    fs::create_dir_all(&config_dir)?;
    let config_path = config_dir.join("config.yaml");

    if matches.get_flag("write_example_config") {
        let example_config = config::build_example_config();
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
    if matches.get_flag("set_keychain_password") {
        println!("Enter your current login user's password to store in keychain:");
        let input = rpassword::read_password()?;
        config.auto_password_input.set_pswd(input.trim())?;
        println!("Success!");
    }
   
    if let Some(mins) = matches.get_one::<usize>("mins") {
        lib::execute_for_duration(&config, Duration::minutes(*mins as i64))?;
    }
    Ok(())
}
