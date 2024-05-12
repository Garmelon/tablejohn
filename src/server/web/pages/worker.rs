use std::sync::{Arc, Mutex};

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use maud::html;
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

    let status = status(&info.status, &db, &base).await?;

    Ok(base
        .html(
            &path.name,
            html! {},
            html! {
                h2 { "Worker" }
                div .commit-like .worker {
                    span .title { "worker " (path.name) }
                    dl {
                        dt { "Connected:" }
                        dd { (util::format_time(info.first_seen)) }

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
                                dd { (link.html()) }

                                dt { "Working since:" }
                                dd { (since) }
                            }
                        }
                    }
                }
            },
        )
        .into_response())
}
