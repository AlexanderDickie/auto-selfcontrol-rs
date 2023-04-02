use std::collections::HashMap;
use std::fmt::{self, Display};
use std::process::Command;
use std::error::Error;
use core_foundation::string::CFString;
use core_foundation::propertylist::{CFPropertyList, CFPropertyListSubClass};
use chrono::{self, NaiveTime, Local};

use super::ResultE;
use super::utils;

#[link(name = "CoreFoundation", kind = "framework")]
extern {
    fn CFPreferencesSetAppValue(key: CFString, value: CFPropertyList, applicationID: CFString);
    fn CFPreferencesAppSynchronize( applicationID: CFString) -> bool;
}

fn selfcontrol_begin_block(self_control_path: &str, duration: chrono::Duration) -> Result<(), SelfControlError> {
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
    let output = Command::new(self_control_path)
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

pub fn selfcontrol_insist_begin_block(self_control_path: &str, end: NaiveTime) -> ResultE<()> {
    /*
    self control requires the user to input their password to install a helper tool,
    if the user refuses, start sc again (the helper prompt will immediately  reappear)
    */
    loop {
        let now = Local::now().time();
        let duration = utils::duration_between(now, end);

        match selfcontrol_begin_block(self_control_path, duration) {
            Ok(_) => return Ok(()),

            Err(e) => match e {
                SelfControlError::UserCancelledHelper => continue,
                _ => return Err(e.into()),

            }
        }
    }

}

fn selfcontrol_parse_settings(stderr: &str) -> Result<HashMap<String, String>, SelfControlCliError> {
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

pub fn selfcontrol_is_active(self_control_path: &str) -> ResultE<Option<NaiveTime>> {
    /*
    checks if self control is currently active, if so returns the time it will end
    */
    let output = Command::new(self_control_path)
        .arg("print-settings")
        .output()?;
    let stderr = String::from_utf8(output.stderr)?;
    let settings_map = selfcontrol_parse_settings(&stderr)?;

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
