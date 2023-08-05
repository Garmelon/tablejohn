use std::sync::Arc;

use askama::Template;
use axum::{
    extract::{Path, State},
    response::IntoResponse,
};
use futures::TryStreamExt;
use gix::{prelude::ObjectIdExt, Id, ObjectId, ThreadSafeRepository};
use sqlx::SqlitePool;

use crate::{config::Config, repo, somehow};

struct Commit {
    hash: String,
    description: String,
}

impl Commit {
    fn new(id: Id<'_>) -> somehow::Result<Self> {
        let commit = id.object()?.try_into_commit()?;
        Ok(Self {
            hash: id.to_string(),
            description: repo::format_commit_short(&commit)?,
        })
    }
}

#[derive(Template)]
#[template(path = "commit_hash.html")]
struct CommitIdTemplate {
    base: String,
    repo_name: String,
    current: String,
    hash: String,
    summary: String,
    message: String,
    author: String,
    author_date: String,
    commit: String,
    commit_date: String,
    parents: Vec<Commit>,
    children: Vec<Commit>,
}

pub async fn get(
    Path(hash): Path<String>,
    State(config): State<&'static Config>,
    State(db): State<SqlitePool>,
    State(repo): State<Arc<ThreadSafeRepository>>,
) -> somehow::Result<impl IntoResponse> {
    // Do this first because a &Repository can't be kept across awaits.
    let child_ids = sqlx::query!("SELECT child FROM commit_links WHERE parent = ?", hash)
        .fetch(&db)
        .map_ok(|r| r.child)
        .try_collect::<Vec<_>>()
        .await?;

    let repo = repo.to_thread_local();
    let id = hash.parse::<ObjectId>()?.attach(&repo);
    let commit = id.object()?.try_into_commit()?;
    let author_info = commit.author()?;
    let committer_info = commit.committer()?;

    let mut parents = vec![];
    for id in commit.parent_ids() {
        parents.push(Commit::new(id)?);
    }

    let mut children = vec![];
    for hash in child_ids {
        let id = hash.parse::<ObjectId>()?.attach(&repo);
        children.push(Commit::new(id)?);
    }

    Ok(CommitIdTemplate {
        base: config.web.base(),
        repo_name: config.repo.name(),
        current: "commit".to_string(),
        hash: id.to_string(),
        summary: commit.message()?.summary().to_string(),
        message: commit.message_raw_sloppy().to_string(),
        author: repo::format_actor(author_info.actor())?,
        author_date: repo::format_time(author_info.time),
        commit: repo::format_actor(committer_info.actor())?,
        commit_date: repo::format_time(committer_info.time),
        parents,
        children,
    })
}
