use chrono::{self, Timelike, NaiveTime, Duration};

pub fn build_launch_agent_plist(
    name: &str,
    command: &str,
    args: &Vec<&str>,
    schedule: &LaunchAgentSchedule,
    run_at_load: bool)
    -> String 
{
    let parts = vec![
        build_plist_header(name),
        build_plist_commands(command, args),
        build_plist_schedule(schedule, run_at_load),
        build_plist_footer(),
    ];
    parts.join("\n")
}

fn build_plist_header(name: &str) -> String {
    format!(
r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{}</string>"#
, name)
}

fn build_plist_commands(command: &str, args: &Vec<&str>) -> String {
    let args = args
        .iter()
        .map(|arg| format!(
r#"        <string>{}</string>"#
            , arg))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
r#"    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
{}
    </array>"#
,command, args)
}

#[allow(dead_code)]
pub enum LaunchAgentSchedule<'a> {
    Calendar(&'a Vec<NaiveTime>), 
    Periodic(Duration), 
}
fn build_plist_schedule(schedule: &LaunchAgentSchedule, run_at_load: bool) -> String {
    let timings = match schedule {
        LaunchAgentSchedule::Periodic(duration) => {
        format!(
r#"    <key>StartInterval</key>
        <integer>{}<\integer>"#
        ,duration.num_seconds())
        },

        LaunchAgentSchedule::Calendar(start_times) => {
            let start_times = start_times
                .iter()
                .map(|time| format!(
r#"       <dict>
            <key>Minute</key>
            <integer>{}</integer>
            <key>Hour</key>
            <integer>{}</integer>
        </dict>"#, 
                time.minute(), time.hour()))
                .collect::<Vec<_>>()
                .join("\n");

            vec![
                "    <key>StartCalendarInterval</key>\n".to_string(),
                "   <array>".to_string(),
                start_times,
                "   </array>".to_string(),
            ].join("\n")
        }
    };
    // if agent is scheduled to execute when computer is shut down, it will not when next online- run at load 
    // to mitigate this
    if run_at_load {
        return timings +
&r#"
    <key>RunAtLoad</key>
    <true/>"#.to_string();
    } else {
        return timings;
    }
}

fn build_plist_footer() -> String {
r#"</dict>
</plist>"#.to_string()
}

