use std::time::Duration;

use gix::date::Time;
use time::{macros::format_description, OffsetDateTime, UtcOffset};

use crate::somehow;

pub fn time_to_offset_datetime(time: Time) -> somehow::Result<OffsetDateTime> {
    Ok(OffsetDateTime::from_unix_timestamp(time.seconds)?
        .to_offset(UtcOffset::from_whole_seconds(time.offset)?))
}

pub fn format_time(time: OffsetDateTime) -> somehow::Result<String> {
    let now = OffsetDateTime::now_utc();
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
