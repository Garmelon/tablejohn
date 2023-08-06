use askama::Template;
use axum::{extract::State, response::IntoResponse};
use futures::TryStreamExt;
use sqlx::SqlitePool;

use crate::{config::Config, db, somehow};

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
    tracked_refs: Vec<Ref>,
}

pub async fn get(
    State(config): State<&'static Config>,
    State(db): State<SqlitePool>,
) -> somehow::Result<impl IntoResponse> {
    let tracked_refs = sqlx::query!(
        "\
        SELECT name, hash, message FROM refs \
        JOIN commits USING (hash) \
        WHERE tracked \
        "
    )
    .fetch(&db)
    .map_ok(|r| Ref {
        name: r.name,
        short: db::format_commit_short(&r.hash, &r.message),
        hash: r.hash,
    })
    .try_collect::<Vec<_>>()
    .await?;

    Ok(IndexTemplate {
        base: config.web.base(),
        repo_name: config.repo.name(),
        current: "index".to_string(),
        tracked_refs,
    })
}
