use std::time::Duration;

use time::{format_description::well_known::Rfc3339, macros::format_description, OffsetDateTime};

use crate::somehow;

pub fn format_time(time: &str) -> somehow::Result<String> {
    let now = OffsetDateTime::now_utc();
    let time = OffsetDateTime::parse(time, &Rfc3339)?;
    let delta = time - now;

    let formatted_time = time.format(format_description!(
        "[year]-[month]-[day] [hour]:[minute]:[second] [offset_hour sign:mandatory][offset_minute]"
    ))?;
    let formatted_delta =
        humantime::format_duration(Duration::from_secs(delta.unsigned_abs().as_secs()));
    Ok(if delta.is_positive() {
        format!("{formatted_time} (in {formatted_delta})")
    } else {
        format!("{formatted_time} ({formatted_delta} ago)")
    })
}

pub fn summary(message: &str) -> String {
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
    let summary = summary(message);
    format!("{short_hash} ({summary})")
}
