//! Utility functions for accessing a [`Repository`].

use gix::{actor::IdentityRef, Commit};

use crate::somehow;

// TODO Remove this function
pub fn format_actor(author: IdentityRef<'_>) -> somehow::Result<String> {
    let mut buffer = vec![];
    author.trim().write_to(&mut buffer)?;
    Ok(String::from_utf8_lossy(&buffer).to_string())
}

// TODO Remove this function
pub fn format_commit_short(commit: &Commit<'_>) -> somehow::Result<String> {
    let id = commit.id().shorten_or_id();
    let summary = commit.message()?.summary();
    Ok(format!("{id} ({summary})"))
}
