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
        base::{Base, Tab},
        paths::{PathAdminQueueAdd, PathQueue},
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

    let link = Base::new(config, Tab::None).link(PathQueue {});
    Ok(Redirect::to(&format!("{link}")))
}
