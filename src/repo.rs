//! Utility functions for accessing a [`Repository`].

use gix::{ObjectId, Repository};

use crate::somehow;

pub fn short_commit(repo: &Repository, hash: &str) -> somehow::Result<String> {
    let hash = hash.parse::<ObjectId>()?;
    let commit = repo.find_object(hash)?.try_into_commit()?;

    let id = commit.short_id()?;
    let summary = commit.message()?.summary();
    Ok(format!("{id} ({summary})"))
}
