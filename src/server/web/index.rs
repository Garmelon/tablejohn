use askama::Template;
use axum::{extract::State, response::IntoResponse};
use futures::TryStreamExt;
use sqlx::SqlitePool;

use crate::{config::Config, somehow};

use super::{
    base::{Base, Tab},
    link::LinkCommit,
    paths::PathIndex,
};

struct Ref {
    name: String,
    commit: LinkCommit,
    tracked: bool,
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    base: Base,
    tracked_refs: Vec<Ref>,
    untracked_refs: Vec<Ref>,
}

pub async fn get_index(
    _path: PathIndex,
    State(config): State<&'static Config>,
    State(db): State<SqlitePool>,
) -> somehow::Result<impl IntoResponse> {
    let base = Base::new(config, Tab::Index);

    let refs = sqlx::query!(
        "\
        SELECT name, hash, message, reachable, tracked \
        FROM refs \
        JOIN commits USING (hash) \
        ORDER BY name ASC \
        "
    )
    .fetch(&db)
    .map_ok(|r| Ref {
        name: r.name.clone(),
        commit: LinkCommit::new(&base, r.hash, &r.message, r.reachable),
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
