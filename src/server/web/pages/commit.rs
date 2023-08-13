use askama::Template;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use futures::TryStreamExt;
use sqlx::SqlitePool;

use crate::{
    config::Config,
    server::{
        util,
        web::{
            base::{Base, Link, Tab},
            link::{LinkCommit, LinkRunDate},
            paths::{PathAdminQueueAdd, PathCommitByHash},
        },
    },
    somehow,
};

#[derive(Template)]
#[template(path = "pages/commit.html")]
struct Page {
    link_admin_queue_add: Link,
    base: Base,

    summary: String,
    hash: String,
    author: String,
    author_date: String,
    commit: String,
    commit_date: String,
    parents: Vec<LinkCommit>,
    children: Vec<LinkCommit>,
    message: String,
    reachable: i64,
    runs: Vec<LinkRunDate>,
}

pub async fn get_commit_by_hash(
    path: PathCommitByHash,
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
        path.hash,
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
        path.hash,
    )
    .fetch(&db)
    .map_ok(|r| LinkCommit::new(&base, r.hash, &r.message, r.reachable))
    .try_collect::<Vec<_>>()
    .await?;

    let children = sqlx::query!(
        "\
        SELECT hash, message, reachable FROM commits \
        JOIN commit_links ON hash = child \
        WHERE parent = ? \
        ORDER BY reachable DESC, unixepoch(committer_date) ASC \
        ",
        path.hash,
    )
    .fetch(&db)
    .map_ok(|r| LinkCommit::new(&base, r.hash, &r.message, r.reachable))
    .try_collect::<Vec<_>>()
    .await?;

    let runs = sqlx::query!(
        "\
        SELECT \
            id, \
            start AS \"start: time::OffsetDateTime\" \
        FROM runs WHERE hash = ? \
        ",
        path.hash
    )
    .fetch(&db)
    .map_ok(|r| LinkRunDate::new(&base, r.id, r.start))
    .try_collect::<Vec<_>>()
    .await?;

    Ok(Page {
        link_admin_queue_add: base.link(PathAdminQueueAdd {}),
        base,

        summary: util::format_commit_summary(&commit.message),
        hash: commit.hash,
        author: commit.author,
        author_date: util::format_time(commit.author_date),
        commit: commit.committer,
        commit_date: util::format_time(commit.committer_date),
        parents,
        children,
        message: commit.message.trim_end().to_string(),
        reachable: commit.reachable,
        runs,
    }
    .into_response())
}
