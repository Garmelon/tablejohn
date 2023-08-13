use std::time::Duration;

use gix::{actor::IdentityRef, date::Time};
use time::{macros::format_description, OffsetDateTime, UtcOffset};

use crate::somehow;

pub fn time_to_offset_datetime(time: Time) -> somehow::Result<OffsetDateTime> {
    Ok(OffsetDateTime::from_unix_timestamp(time.seconds)?
        .to_offset(UtcOffset::from_whole_seconds(time.offset)?))
}

pub fn format_duration(duration: time::Duration) -> String {
    let seconds = duration.unsigned_abs().as_secs(); // To nearest second
    let formatted = humantime::format_duration(Duration::from_secs(seconds));
    format!("{formatted}")
}

pub fn format_delta_from_now(time: OffsetDateTime) -> String {
    let now = OffsetDateTime::now_utc();
    let delta = time - now;
    let seconds = delta.unsigned_abs().as_secs();
    let seconds = seconds + 30 - (seconds + 30) % 60; // To nearest minute
    if seconds == 0 {
        return "now".to_string();
    }
    let formatted = humantime::format_duration(Duration::from_secs(seconds));
    if delta.is_positive() {
        format!("in {formatted}")
    } else {
        format!("{formatted} ago")
    }
}

pub fn format_time(time: OffsetDateTime) -> String {
    let formatted_time = time
        .format(format_description!(
        "[year]-[month]-[day] [hour]:[minute]:[second] [offset_hour sign:mandatory][offset_minute]"
    ))
        .expect("invalid date format");

    let formatted_delta = format_delta_from_now(time);
    format!("{formatted_time} ({formatted_delta})")
}

pub fn format_actor(author: IdentityRef<'_>) -> somehow::Result<String> {
    let mut buffer = vec![];
    author.trim().write_to(&mut buffer)?;
    Ok(String::from_utf8_lossy(&buffer).to_string())
}

pub fn format_commit_summary(message: &str) -> String {
    // Take everything up to the first double newline
    let title = message
        .split_once("\n\n")
        .map(|(t, _)| t)
        .unwrap_or(message);

    // Turn consecutive whitespace into a single space
    title.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn format_commit_short(hash: &str, message: &str) -> String {
    let short_hash = hash.chars().take(8).collect::<String>();
    let summary = format_commit_summary(message);
    format!("{short_hash} ({summary})")
}

pub fn format_value(value: f64) -> String {
    if value.abs() >= 1e6 {
        format!("{value:.3e}")
    } else if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        format!("{value:.2}")
    }
}
