use askama::Template;
use axum::{extract::State, response::IntoResponse};
use futures::TryStreamExt;
use sqlx::SqlitePool;

use crate::{config::Config, somehow, util};

use super::{Base, Tab};

struct Task {
    id: String,
    short: String,
    since: String,
    priority: i64,
}

async fn get_queue(db: &SqlitePool) -> somehow::Result<Vec<Task>> {
    sqlx::query!(
        "\
        SELECT \
            id, \
            hash, \
            message, \
            date AS \"date: time::OffsetDateTime\", \
            priority \
        FROM queue \
        JOIN commits USING (hash) \
        ORDER BY priority DESC, unixepoch(date) DESC, hash ASC \
        "
    )
    .fetch(db)
    .map_ok(|r| Task {
        id: r.id,
        short: util::format_commit_short(&r.hash, &r.message),
        since: util::format_delta_from_now(r.date),
        priority: r.priority,
    })
    .err_into::<somehow::Error>()
    .try_collect::<Vec<_>>()
    .await
}

#[derive(Template)]
#[template(path = "queue_table.html")]
struct QueueTableTemplate {
    tasks: Vec<Task>,
}

pub async fn get_table(State(db): State<SqlitePool>) -> somehow::Result<impl IntoResponse> {
    let tasks = get_queue(&db).await?;
    Ok(QueueTableTemplate { tasks })
}
#[derive(Template)]
#[template(path = "queue.html")]
struct QueueTemplate {
    base: Base,
    table: QueueTableTemplate,
}

pub async fn get(
    State(config): State<&'static Config>,
    State(db): State<SqlitePool>,
) -> somehow::Result<impl IntoResponse> {
    let tasks = get_queue(&db).await?;
    Ok(QueueTemplate {
        base: Base::new(config, Tab::Queue),
        table: QueueTableTemplate { tasks },
    })
}
