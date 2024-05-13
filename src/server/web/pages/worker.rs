use std::sync::{Arc, Mutex};

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use maud::{html, Markup};
use sqlx::SqlitePool;

use crate::{
    config::ServerConfig,
    server::{
        format,
        web::{components, page::Page, paths::PathWorkerByName},
        workers::Workers,
    },
    shared::WorkerStatus,
    somehow,
};

enum Status {
    Idle,
    Busy,
    Working { link: Markup, since: String },
}

async fn status(
    config: &ServerConfig,
    status: &WorkerStatus,
    db: &SqlitePool,
) -> somehow::Result<Status> {
    Ok(match status {
        WorkerStatus::Idle => Status::Idle,
        WorkerStatus::Busy => Status::Busy,
        WorkerStatus::Working(run) => {
            let message =
                sqlx::query_scalar!("SELECT message FROM commits WHERE hash = ?", run.hash)
                    .fetch_one(db)
                    .await?;
            Status::Working {
                link: components::link_run_short(config, run.id.clone(), &run.hash, &message),
                since: format::time(run.start.0),
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

    let status = status(config, &info.status, &db).await?;

    let html = Page::new(config)
        .title(&path.name)
        .body(html! {
            h2 { "Worker" }
            div .commit-like .worker {
                span .title { "worker " (path.name) }
                dl {
                    dt { "Connected:" }
                    dd { (format::time(info.first_seen)) }

                    @match status {
                        Status::Idle => {
                            dt { "Working on:" }
                            dd { "nothing" }
                        }
                        Status::Busy => {
                            dt { "Working on:" }
                            dd { "run for another server" }
                        }
                        Status::Working { link, since } => {
                            dt { "Working on:" }
                            dd { (link) }

                            dt { "Working since:" }
                            dd { (since) }
                        }
                    }
                }
            }
        })
        .build();

    Ok(html.into_response())
}
