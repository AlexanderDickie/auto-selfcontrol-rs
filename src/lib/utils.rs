use chrono::{Duration, NaiveTime};
use std::error::Error;

pub fn duration_between(start: NaiveTime, end: NaiveTime) -> Duration {
    let dif = end - start;
    match start < end {
        true => dif,
        false => Duration::hours(24) + dif,
    }
}

pub type ResultE<T> = std::result::Result<T, Box<dyn Error>>;
