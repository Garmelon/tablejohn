use askama::Template;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use futures::TryStreamExt;
use sqlx::SqlitePool;

use crate::{config::Config, db, somehow};

struct Commit {
    hash: String,
    short: String,
    reachable: i64,
}

impl Commit {
    fn new(hash: String, message: &str, reachable: i64) -> Self {
        Self {
            short: db::format_commit_short(&hash, message),
            hash,
            reachable,
        }
    }
}

#[derive(Template)]
#[template(path = "commit_hash.html")]
struct CommitIdTemplate {
    base: String,
    repo_name: String,
    current: String,
    hash: String,
    author: String,
    author_date: String,
    commit: String,
    commit_date: String,
    parents: Vec<Commit>,
    children: Vec<Commit>,
    summary: String,
    message: String,
    reachable: i64,
}

pub async fn get(
    Path(hash): Path<String>,
    State(config): State<&'static Config>,
    State(db): State<SqlitePool>,
) -> somehow::Result<Response> {
    let Some(commit) = sqlx::query!("SELECT * FROM commits WHERE hash = ?", hash)
        .fetch_optional(&db)
        .await?
    else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };

    let parents = sqlx::query!(
        "\
        SELECT hash, message, reachable FROM commits \
        JOIN commit_links ON hash = parent \
        WHERE child = ? \
        ",
        hash
    )
    .fetch(&db)
    .map_ok(|r| Commit::new(r.hash, &r.message, r.reachable))
    .try_collect::<Vec<_>>()
    .await?;

    let children = sqlx::query!(
        "\
        SELECT hash, message, reachable FROM commits \
        JOIN commit_links ON hash = child \
        WHERE parent = ? \
        ",
        hash
    )
    .fetch(&db)
    .map_ok(|r| Commit::new(r.hash, &r.message, r.reachable))
    .try_collect::<Vec<_>>()
    .await?;

    Ok(CommitIdTemplate {
        base: config.web.base(),
        repo_name: config.repo.name(),
        current: "commit".to_string(),
        hash: commit.hash,
        author: commit.author,
        author_date: db::format_time(&commit.author_date)?,
        commit: commit.committer,
        commit_date: db::format_time(&commit.committer_date)?,
        parents,
        children,
        summary: db::summary(&commit.message),
        message: commit.message.trim_end().to_string(),
        reachable: commit.reachable,
    }
    .into_response())
}
