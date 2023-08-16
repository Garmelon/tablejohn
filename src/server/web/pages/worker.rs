use std::sync::{Arc, Mutex};

use askama::Template;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sqlx::SqlitePool;

use crate::{
    config::ServerConfig,
    server::{
        util,
        web::{
            base::{Base, Tab},
            link::LinkRunShort,
            paths::PathWorkerByName,
        },
        workers::Workers,
    },
    shared::WorkerStatus,
    somehow,
};

enum Status {
    Idle,
    Busy,
    Working { link: LinkRunShort, since: String },
}

#[derive(Template)]
#[template(path = "pages/worker.html")]
struct Page {
    base: Base,

    name: String,
    connected: String,
    status: Status,
}

async fn status(status: &WorkerStatus, db: &SqlitePool, base: &Base) -> somehow::Result<Status> {
    Ok(match status {
        WorkerStatus::Idle => Status::Idle,
        WorkerStatus::Busy => Status::Busy,
        WorkerStatus::Working(run) => {
            let message =
                sqlx::query_scalar!("SELECT message FROM commits WHERE hash = ?", run.hash)
                    .fetch_one(db)
                    .await?;
            Status::Working {
                link: LinkRunShort::new(base, run.id.clone(), &run.hash, &message),
                since: util::format_time(run.start.0),
            }
        }
    })
}

pub async fn get_worker_by_name(
    path: PathWorkerByName,
    State(config): State<&'static ServerConfig>,
    State(db): State<SqlitePool>,
    State(workers): State<Arc<Mutex<Workers>>>,
) -> somehow::Result<Response> {
    let info = workers.lock().unwrap().clean().get(&path.name);
    let Some(info) = info else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };

    let base = Base::new(config, Tab::None);
    Ok(Page {
        name: path.name,
        connected: util::format_time(info.first_seen),
        status: status(&info.status, &db, &base).await?,

        base,
    }
    .into_response())
}
