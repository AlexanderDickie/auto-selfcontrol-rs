use chrono::NaiveDateTime;
use {
    chrono::{self, Local, NaiveTime},
    core_foundation::{
        propertylist::{CFPropertyList, CFPropertyListSubClass},
        string::CFString,
    },
};
use {
    std::{collections::HashMap, error::Error, fmt::{self, Display}, path::PathBuf, process::{Command, Output}},
    tokio::process::Command as TokioCommand,
};
use super::ResultE;


#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFPreferencesSetAppValue(key: CFString, value: CFPropertyList, applicationID: CFString);
    fn CFPreferencesAppSynchronize(applicationID: CFString) -> bool;
}

pub async fn start_sc_until(
    selfcontrol_path: &PathBuf,
    end: NaiveDateTime,
) -> Result<(), SelfControlError> {
    let now = Local::now().naive_local();

    if now >= end {
        return Ok(());
    }
    let duration = super::super::utils::duration_between(now.time(), end.time());

    set_block_duration(duration);

    let start_self_control = TokioCommand::new(selfcontrol_path)
        .arg("start")
        .kill_on_drop(true)
        .output();

    let result = tokio::time::timeout(tokio::time::Duration::from_secs(5), start_self_control).await;

    let self_control_output = match result {
        Err(_) => return Err(SelfControlError::NoInputTimeout.into()),
        Ok(result) => result,
    }
    .map_err(|e| SelfControlError::CommandError(e.to_string()))?;

    parse_self_control_output(self_control_output)
}

fn set_block_duration(duration: chrono::Duration) {
    let mins = (duration.num_seconds() as f64 / 60.0).ceil() as u32;
    unsafe {
        CFPreferencesSetAppValue(
            CFString::new("BlockDuration"),
            CFString::new(&mins.to_string()).to_CFPropertyList(),
            CFString::new("org.eyebeam.SelfControl"),
        );
        CFPreferencesAppSynchronize(CFString::new("org.eyebeam.SelfControl"));
    }
}

fn parse_self_control_output(output: Output) -> Result<(), SelfControlError> {
    use SelfControlError::*;
    let stderr = String::from_utf8(output.stderr).map_err(|e| ParseError(e))?;

    if stderr.contains("Authorization cancelled") {
        return Err(UserCancelledHelper.into());
    }

    if !stderr.contains(&"INFO: Block successfully added.") {
        return Err(NoSuccessMsg.into());
    }
    Ok(())
}

fn parse_settings(stderr: &str) -> Result<HashMap<String, String>, SelfControlCliError> {
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

pub fn is_active(self_control_path: &PathBuf) -> ResultE<Option<NaiveTime>> {
    let output = Command::new(self_control_path)
        .arg("print-settings")
        .output()?;
    let stderr = String::from_utf8(output.stderr)?;
    let settings_map = parse_settings(&stderr)?;

    let is_active = settings_map
        .get("BlockIsRunning")
        .ok_or(SelfControlCliError::MisingPlistKey)?;

    if is_active == "0" {
        return Ok(None);
    }

    let end_date = settings_map
        .get("BlockEndDate")
        .ok_or(SelfControlCliError::MisingPlistKey)?;

    // our date value has weird format- "\"2022-12-3022:25:27+0000\"" so format it
    let end_date = end_date.replace("\"", "");
    let end_date = &end_date[10..end_date.chars().count() - 5];
    let end_date = NaiveTime::parse_from_str(end_date, "%H:%M:%S")?;
    Ok(Some(end_date))
}

#[derive(Debug)]
pub enum SelfControlError {
    UserCancelledHelper,
    NoInputTimeout,
    NoSuccessMsg,
    CommandError(String),
    ParseError(std::string::FromUtf8Error),
}
impl Display for SelfControlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SelfControlError::UserCancelledHelper => {
                write!(f, "User cancelled helper authorization")
            }
            SelfControlError::NoSuccessMsg => {
                write!(f, "no success message in SelfControl stderr output")
            }
            SelfControlError::CommandError(e) => {
                write!(f, "error executing self control binary: {}", e)
            }
            SelfControlError::ParseError(e) => {
                write!(f, "error parsing self control output: {}", e)
            }
            Self::NoInputTimeout => write!(f, "self control not started within period: "),
        }
    }
}
impl Error for SelfControlError {}

#[derive(Debug)]
enum SelfControlCliError {
    InvalidPlistFormat,
    MisingPlistKey,
}
impl Display for SelfControlCliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SelfControlCliError::MisingPlistKey => {
                write!(f, "missing key in SelfControl cli plist")
            }
            SelfControlCliError::InvalidPlistFormat => write!(f, "invalid SelfControl cli plist"),
        }
    }
}
impl Error for SelfControlCliError {}
