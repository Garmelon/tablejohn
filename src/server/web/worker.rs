use std::sync::{Arc, Mutex};

use askama::Template;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{
    config::Config,
    server::{util, workers::Workers},
    somehow,
};

use super::{
    base::{Base, Tab},
    paths::PathWorkerByName,
};

#[derive(Template)]
#[template(path = "worker.html")]
struct WorkerTemplate {
    base: Base,
    name: String,
    last_seen: String,
    // TODO Status
}

pub async fn get_worker_by_name(
    path: PathWorkerByName,
    State(config): State<&'static Config>,
    State(workers): State<Arc<Mutex<Workers>>>,
) -> somehow::Result<Response> {
    let info = workers.lock().unwrap().clean().get(&path.name);
    let Some(info) = info else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };

    Ok(WorkerTemplate {
        base: Base::new(config, Tab::None),
        name: path.name,
        last_seen: util::format_time(info.last_seen),
    }
    .into_response())
}
