use std::sync::{Arc, Mutex};

use askama::Template;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::{
    config::Config,
    server::{runners::Runners, util},
    somehow,
};

use super::{Base, Tab};

#[derive(Template)]
#[template(path = "runner.html")]
struct RunnerTemplate {
    base: Base,
    name: String,
    last_seen: String,
    // TODO Status
}

pub async fn get(
    Path(name): Path<String>,
    State(config): State<&'static Config>,
    State(runners): State<Arc<Mutex<Runners>>>,
) -> somehow::Result<Response> {
    let info = runners.lock().unwrap().clean().get(&name);
    let Some(info) = info else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };

    Ok(RunnerTemplate {
        base: Base::new(config, Tab::None),
        name,
        last_seen: util::format_time(info.last_seen),
    }
    .into_response())
}
