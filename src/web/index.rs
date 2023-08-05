use std::sync::Arc;

use askama::Template;
use axum::{extract::State, response::IntoResponse};
use gix::ThreadSafeRepository;
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
        let name = row.name;
        let hash = row.hash;
        let short = repo::short_commit(&repo, &hash)?;
        refs.push(Ref { name, hash, short });
    }

    Ok(IndexTemplate {
        base: config.web.base(),
        repo_name: config.repo.name(),
        refs,
    })
}
