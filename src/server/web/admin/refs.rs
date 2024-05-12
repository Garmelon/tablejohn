use std::sync::Arc;

use axum::{
    extract::State,
    response::{IntoResponse, Redirect},
    Form,
};
use log::info;
use serde::Deserialize;
use sqlx::SqlitePool;
use tokio::sync::mpsc;

use crate::{
    config::ServerConfig,
    server::web::{
        paths::{PathAdminRefsTrack, PathAdminRefsUntrack, PathAdminRefsUpdate, PathIndex},
        server_config_ext::ServerConfigExt,
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

    Ok(Redirect::to(config.path(PathIndex {}).as_ref()))
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

    Ok(Redirect::to(config.path(PathIndex {}).as_ref()))
}

pub async fn post_admin_repo_update(
    _path: PathAdminRefsUpdate,
    State(config): State<&'static ServerConfig>,
    State(recurring_tx): State<Arc<mpsc::UnboundedSender<()>>>,
) -> somehow::Result<impl IntoResponse> {
    let _ = recurring_tx.send(());
    info!("Admin updated repo");

    Ok(Redirect::to(config.path(PathIndex {}).as_ref()))
}
