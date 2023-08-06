use askama::Template;
use axum::{extract::State, response::IntoResponse};
use futures::TryStreamExt;
use sqlx::SqlitePool;

use crate::{config::Config, somehow, util};

use super::{Base, Tab};

struct Ref {
    name: String,
    hash: String,
    short: String,
    tracked: bool,
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    base: Base,
    tracked_refs: Vec<Ref>,
    untracked_refs: Vec<Ref>,
}

pub async fn get(
    State(config): State<&'static Config>,
    State(db): State<SqlitePool>,
) -> somehow::Result<impl IntoResponse> {
    let refs = sqlx::query!(
        "\
        SELECT name, hash, tracked, message FROM refs \
        JOIN commits USING (hash) \
        ORDER BY name ASC \
        "
    )
    .fetch(&db)
    .map_ok(|r| Ref {
        name: r.name,
        short: util::format_commit_short(&r.hash, &r.message),
        hash: r.hash,
        tracked: r.tracked != 0,
    })
    .try_collect::<Vec<_>>()
    .await?;

    let mut tracked_refs = vec![];
    let mut untracked_refs = vec![];
    for reference in refs {
        if reference.tracked {
            tracked_refs.push(reference);
        } else {
            untracked_refs.push(reference);
        }
    }

    Ok(IndexTemplate {
        base: Base::new(config, Tab::Index),
        tracked_refs,
        untracked_refs,
    })
}
