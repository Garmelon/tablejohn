use axum::{
    extract::State,
    response::{IntoResponse, Redirect},
    Form,
};
use serde::Deserialize;
use sqlx::SqlitePool;
use time::OffsetDateTime;

use crate::{
    config::Config,
    server::web::{
        base::Base,
        paths::{
            PathAdminQueueAdd, PathAdminQueueDecrease, PathAdminQueueDelete,
            PathAdminQueueIncrease, PathQueue,
        },
    },
    somehow,
};

#[derive(Deserialize)]
pub struct FormAdminQueueAdd {
    hash: String,
    #[serde(default)]
    priority: i32,
}

pub async fn post_admin_queue_add(
    _path: PathAdminQueueAdd,
    State(config): State<&'static Config>,
    State(db): State<SqlitePool>,
    Form(form): Form<FormAdminQueueAdd>,
) -> somehow::Result<impl IntoResponse> {
    let date = OffsetDateTime::now_utc();
    sqlx::query!(
        "\
        INSERT INTO queue (hash, date, priority) VALUES (?, ?, ?) \
        ON CONFLICT (hash) DO UPDATE \
        SET priority = excluded.priority WHERE priority < excluded.priority \
        ",
        form.hash,
        date,
        form.priority,
    )
    .execute(&db)
    .await?;

    let link = Base::link_with_config(config, PathQueue {});
    Ok(Redirect::to(&format!("{link}")))
}

#[derive(Deserialize)]
pub struct FormAdminQueueDelete {
    hash: String,
}

pub async fn post_admin_queue_delete(
    _path: PathAdminQueueDelete,
    State(config): State<&'static Config>,
    State(db): State<SqlitePool>,
    Form(form): Form<FormAdminQueueDelete>,
) -> somehow::Result<impl IntoResponse> {
    sqlx::query!("DELETE FROM queue WHERE hash = ?", form.hash)
        .execute(&db)
        .await?;

    let link = Base::link_with_config(config, PathQueue {});
    Ok(Redirect::to(&format!("{link}")))
}

#[derive(Deserialize)]
pub struct FormAdminQueueIncrease {
    hash: String,
}

pub async fn post_admin_queue_increase(
    _path: PathAdminQueueIncrease,
    State(config): State<&'static Config>,
    State(db): State<SqlitePool>,
    Form(form): Form<FormAdminQueueIncrease>,
) -> somehow::Result<impl IntoResponse> {
    sqlx::query!(
        "UPDATE queue SET priority = priority + 1 WHERE hash = ?",
        form.hash
    )
    .execute(&db)
    .await?;

    let link = Base::link_with_config(config, PathQueue {});
    Ok(Redirect::to(&format!("{link}")))
}

#[derive(Deserialize)]
pub struct FormAdminQueueDecrease {
    hash: String,
}

pub async fn post_admin_queue_decrease(
    _path: PathAdminQueueDecrease,
    State(config): State<&'static Config>,
    State(db): State<SqlitePool>,
    Form(form): Form<FormAdminQueueDecrease>,
) -> somehow::Result<impl IntoResponse> {
    sqlx::query!(
        "UPDATE queue SET priority = priority - 1 WHERE hash = ?",
        form.hash
    )
    .execute(&db)
    .await?;

    let link = Base::link_with_config(config, PathQueue {});
    Ok(Redirect::to(&format!("{link}")))
}
