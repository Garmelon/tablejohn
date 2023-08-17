use axum::{
    extract::State,
    response::{IntoResponse, Redirect},
    Form,
};
use log::info;
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::{
    config::ServerConfig,
    server::web::{
        base::Base,
        paths::{PathAdminRefsTrack, PathAdminRefsUntrack, PathIndex},
    },
    somehow,
};

#[derive(Deserialize)]
pub struct FormAdminRefsTrack {
    r#ref: String,
}

pub async fn post_admin_refs_track(
    _path: PathAdminRefsTrack,
    State(config): State<&'static ServerConfig>,
    State(db): State<SqlitePool>,
    Form(form): Form<FormAdminRefsTrack>,
) -> somehow::Result<impl IntoResponse> {
    let result = sqlx::query!("UPDATE refs SET tracked = 1 WHERE name = ?", form.r#ref)
        .execute(&db)
        .await?;

    if result.rows_affected() > 0 {
        info!("Admin tracked {}", form.r#ref);
    }

    let link = Base::link_with_config(config, PathIndex {});
    Ok(Redirect::to(&link.to_string()))
}

pub async fn post_admin_refs_untrack(
    _path: PathAdminRefsUntrack,
    State(config): State<&'static ServerConfig>,
    State(db): State<SqlitePool>,
    Form(form): Form<FormAdminRefsTrack>,
) -> somehow::Result<impl IntoResponse> {
    let result = sqlx::query!("UPDATE refs SET tracked = 0 WHERE name = ?", form.r#ref)
        .execute(&db)
        .await?;

    if result.rows_affected() > 0 {
        info!("Admin untracked {}", form.r#ref);
    }

    let link = Base::link_with_config(config, PathIndex {});
    Ok(Redirect::to(&link.to_string()))
}
