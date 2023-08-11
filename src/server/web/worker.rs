use std::sync::{Arc, Mutex};

use askama::Template;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{
    config::Config,
    server::{util, workers::Workers},
    somehow,
};

use super::{Base, Tab};

#[derive(Template)]
#[template(path = "worker.html")]
struct WorkerTemplate {
    base: Base,
    name: String,
    last_seen: String,
    // TODO Status
}

pub async fn get(
    Path(name): Path<String>,
    State(config): State<&'static Config>,
    State(workers): State<Arc<Mutex<Workers>>>,
) -> somehow::Result<Response> {
    let info = workers.lock().unwrap().clean().get(&name);
    let Some(info) = info else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };

    Ok(WorkerTemplate {
        base: Base::new(config, Tab::None),
        name,
        last_seen: util::format_time(info.last_seen),
    }
    .into_response())
}
