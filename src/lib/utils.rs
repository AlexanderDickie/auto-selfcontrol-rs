use chrono::{NaiveTime, Duration};

pub fn duration_between(start: NaiveTime, end: NaiveTime) -> Duration {
    let dif = end - start;
    match start < end {
        true => dif,
        false => Duration::hours(24) + dif, 
    }
} 
