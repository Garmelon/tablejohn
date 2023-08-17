use axum::{
    extract::State,
    response::{IntoResponse, Redirect},
    Form,
};
use log::info;
use serde::Deserialize;
use sqlx::SqlitePool;
use time::OffsetDateTime;

use crate::{
    config::ServerConfig,
    server::web::{
        base::Base,
        paths::{
            PathAdminQueueAdd, PathAdminQueueAddBatch, PathAdminQueueDecrease,
            PathAdminQueueDelete, PathAdminQueueIncrease, PathQueue,
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
    State(config): State<&'static ServerConfig>,
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

    info!(
        "Admin added {} to queue with priority {}",
        form.hash, form.priority,
    );

    let link = Base::link_with_config(config, PathQueue {});
    Ok(Redirect::to(&format!("{link}")))
}

#[derive(Deserialize)]
pub struct FormAdminQueueAddBatch {
    amount: u32,
    #[serde(default)]
    priority: i32,
}

pub async fn post_admin_queue_add_batch(
    _path: PathAdminQueueAddBatch,
    State(config): State<&'static ServerConfig>,
    State(db): State<SqlitePool>,
    Form(form): Form<FormAdminQueueAddBatch>,
) -> somehow::Result<impl IntoResponse> {
    let date = OffsetDateTime::now_utc();
    let added = sqlx::query!(
        "\
        INSERT OR IGNORE INTO queue (hash, date, priority) \
        SELECT hash, ?, ? \
        FROM commits \
        LEFT JOIN runs USING (hash) \
        WHERE reachable = 2 AND id IS NULL \
        ORDER BY unixepoch(committer_date) DESC \
        LIMIT ? \
        ",
        date,
        form.priority,
        form.amount,
    )
    .execute(&db)
    .await?
    .rows_affected();

    if added > 0 {
        info!(
            "Admin batch-added {added} commits to queue with priority {}",
            form.priority,
        );
    }

    let link = Base::link_with_config(config, PathQueue {});
    Ok(Redirect::to(&format!("{link}")))
}

#[derive(Deserialize)]
pub struct FormAdminQueueDelete {
    hash: String,
}

pub async fn post_admin_queue_delete(
    _path: PathAdminQueueDelete,
    State(config): State<&'static ServerConfig>,
    State(db): State<SqlitePool>,
    Form(form): Form<FormAdminQueueDelete>,
) -> somehow::Result<impl IntoResponse> {
    sqlx::query!("DELETE FROM queue WHERE hash = ?", form.hash)
        .execute(&db)
        .await?;

    info!("Admin deleted {} from queue", form.hash);

    let link = Base::link_with_config(config, PathQueue {});
    Ok(Redirect::to(&format!("{link}")))
}

#[derive(Deserialize)]
pub struct FormAdminQueueIncrease {
    hash: String,
}

pub async fn post_admin_queue_increase(
    _path: PathAdminQueueIncrease,
    State(config): State<&'static ServerConfig>,
    State(db): State<SqlitePool>,
    Form(form): Form<FormAdminQueueIncrease>,
) -> somehow::Result<impl IntoResponse> {
    sqlx::query!(
        "UPDATE queue SET priority = priority + 1 WHERE hash = ?",
        form.hash
    )
    .execute(&db)
    .await?;

    info!("Admin increased queue priority of {} by one", form.hash);

    let link = Base::link_with_config(config, PathQueue {});
    Ok(Redirect::to(&format!("{link}")))
}

#[derive(Deserialize)]
pub struct FormAdminQueueDecrease {
    hash: String,
}

pub async fn post_admin_queue_decrease(
    _path: PathAdminQueueDecrease,
    State(config): State<&'static ServerConfig>,
    State(db): State<SqlitePool>,
    Form(form): Form<FormAdminQueueDecrease>,
) -> somehow::Result<impl IntoResponse> {
    sqlx::query!(
        "UPDATE queue SET priority = priority - 1 WHERE hash = ?",
        form.hash
    )
    .execute(&db)
    .await?;

    info!("Admin decreased queue priority of {} by one", form.hash);

    let link = Base::link_with_config(config, PathQueue {});
    Ok(Redirect::to(&format!("{link}")))
}
