use std::sync::Arc;

use askama::Template;
use axum::{extract::State, response::IntoResponse};
use gix::{prelude::ObjectIdExt, ObjectId, ThreadSafeRepository};
use sqlx::SqlitePool;

use crate::{config::Config, repo, somehow};

struct Ref {
    name: String,
    hash: String,
    short: String,
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    base: String,
    repo_name: String,
    current: String,
    refs: Vec<Ref>,
}

pub async fn get(
    State(config): State<&'static Config>,
    State(db): State<SqlitePool>,
    State(repo): State<Arc<ThreadSafeRepository>>,
) -> somehow::Result<impl IntoResponse> {
    let repo = repo.to_thread_local();

    let rows = sqlx::query!("SELECT name, hash FROM tracked_refs")
        .fetch_all(&db)
        .await?;

    let mut refs = vec![];
    for row in rows {
        let id = row.hash.parse::<ObjectId>()?.attach(&repo);
        let commit = id.object()?.try_into_commit()?;

        refs.push(Ref {
            name: row.name,
            hash: row.hash,
            short: repo::format_commit_short(&commit)?,
        });
    }

    Ok(IndexTemplate {
        base: config.web.base(),
        repo_name: config.repo.name(),
        current: "index".to_string(),
        refs,
    })
}
