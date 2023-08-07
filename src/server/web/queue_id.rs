use askama::Template;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sqlx::SqlitePool;

use crate::{config::Config, server::util, somehow};

use super::{Base, Tab};

#[derive(Template)]
#[template(path = "queue_id.html")]
struct QueueIdTemplate {
    base: Base,
    // Task
    id: String,
    hash: String,
    date: String,
    priority: i64,
    // Commit
    summary: String,
    short: String,
    reachable: i64,
}

pub async fn get(
    Path(id): Path<String>,
    State(config): State<&'static Config>,
    State(db): State<SqlitePool>,
) -> somehow::Result<Response> {
    let Some(task) = sqlx::query!(
        "\
        SELECT \
            id, \
            hash, \
            date AS \"date: time::OffsetDateTime\", \
            priority, \
            message, \
            reachable \
        FROM queue \
        JOIN commits USING (hash) \
        WHERE id = ? \
        ",
        id
    )
    .fetch_optional(&db)
    .await?
    else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };

    Ok(QueueIdTemplate {
        base: Base::new(config, Tab::Queue),
        date: util::format_time(task.date),
        id: task.id,
        priority: task.priority,
        summary: util::format_commit_summary(&task.message),
        short: util::format_commit_short(&task.hash, &task.message),
        hash: task.hash,
        reachable: task.reachable,
    }
    .into_response())
}
