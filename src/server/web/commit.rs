use askama::Template;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
};
use futures::TryStreamExt;
use sqlx::SqlitePool;
use time::OffsetDateTime;

use crate::{config::Config, server::util, somehow};

use super::{link::CommitLink, Base, Tab};

#[derive(Template)]
#[template(path = "commit.html")]
struct CommitTemplate {
    base: Base,
    hash: String,
    author: String,
    author_date: String,
    commit: String,
    commit_date: String,
    parents: Vec<CommitLink>,
    children: Vec<CommitLink>,
    summary: String,
    message: String,
    reachable: i64,
}

pub async fn get(
    Path(hash): Path<String>,
    State(config): State<&'static Config>,
    State(db): State<SqlitePool>,
) -> somehow::Result<Response> {
    let base = Base::new(config, Tab::None);

    let Some(commit) = sqlx::query!(
        "\
        SELECT \
            hash, \
            author, \
            author_date AS \"author_date: time::OffsetDateTime\", \
            committer, \
            committer_date AS \"committer_date: time::OffsetDateTime\", \
            message, \
            reachable \
        FROM commits \
        WHERE hash = ? \
        ",
        hash
    )
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
        ORDER BY reachable DESC, unixepoch(committer_date) ASC \
        ",
        hash
    )
    .fetch(&db)
    .map_ok(|r| CommitLink::new(&base, r.hash, &r.message, r.reachable))
    .try_collect::<Vec<_>>()
    .await?;

    let children = sqlx::query!(
        "\
        SELECT hash, message, reachable FROM commits \
        JOIN commit_links ON hash = child \
        WHERE parent = ? \
        ORDER BY reachable DESC, unixepoch(committer_date) ASC \
        ",
        hash
    )
    .fetch(&db)
    .map_ok(|r| CommitLink::new(&base, r.hash, &r.message, r.reachable))
    .try_collect::<Vec<_>>()
    .await?;

    Ok(CommitTemplate {
        base,
        hash: commit.hash,
        author: commit.author,
        author_date: util::format_time(commit.author_date),
        commit: commit.committer,
        commit_date: util::format_time(commit.committer_date),
        parents,
        children,
        summary: util::format_commit_summary(&commit.message),
        message: commit.message.trim_end().to_string(),
        reachable: commit.reachable,
    }
    .into_response())
}

pub async fn post_enqueue(
    Path(hash): Path<String>,
    State(config): State<&'static Config>,
    State(db): State<SqlitePool>,
) -> somehow::Result<impl IntoResponse> {
    let date = OffsetDateTime::now_utc();
    sqlx::query!(
        "INSERT OR IGNORE INTO queue (hash, date, priority) VALUES (?, ?, 1)",
        hash,
        date,
    )
    .execute(&db)
    .await?;

    Ok(Redirect::to(&format!("{}queue/", config.web_base)))
}
