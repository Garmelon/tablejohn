use std::time::Duration;

use gix::actor::IdentityRef;
use time::{macros::format_description, OffsetDateTime};

use crate::{primitive::Timestamp, somehow};

pub fn duration(duration: time::Duration) -> String {
    let seconds = duration.unsigned_abs().as_secs(); // To nearest second
    let formatted = humantime::format_duration(Duration::from_secs(seconds));
    format!("{formatted}")
}

pub fn delta_from_now(time: Timestamp) -> String {
    let now = OffsetDateTime::now_utc();
    let delta = time.0 - now;
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

pub fn time(time: Timestamp) -> String {
    let formatted_time = time
        .0
        .format(format_description!(
        "[year]-[month]-[day] [hour]:[minute]:[second] [offset_hour sign:mandatory][offset_minute]"
    ))
        .expect("invalid date format");

    let formatted_delta = delta_from_now(time);
    format!("{formatted_time} ({formatted_delta})")
}

pub fn actor(author: IdentityRef<'_>) -> somehow::Result<String> {
    let mut buffer = vec![];
    author.trim().write_to(&mut buffer)?;
    Ok(String::from_utf8_lossy(&buffer).to_string())
}

pub fn commit_summary(message: &str) -> String {
    // Take everything up to the first double newline
    let title = message
        .split_once("\n\n")
        .map(|(t, _)| t)
        .unwrap_or(message);

    // Turn consecutive whitespace into a single space
    title.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn commit_short(hash: &str, message: &str) -> String {
    let short_hash = hash.chars().take(8).collect::<String>();
    let summary = commit_summary(message);
    format!("{short_hash} ({summary})")
}

pub fn measurement_value(value: f64) -> String {
    if value.abs() >= 1e6 {
        format!("{value:.3e}")
    } else if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        format!("{value:.2}")
    }
}

pub fn truncate(text: &str, width: usize) -> String {
    let truncate = text.chars().take(width + 1).count() > width;
    if truncate {
        text.chars()
            .take(80 - 3)
            .chain("...".chars())
            .collect::<String>()
    } else {
        text.to_string()
    }
}
