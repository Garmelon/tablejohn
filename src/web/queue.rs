use askama::Template;
use axum::{extract::State, response::IntoResponse};

use crate::{config::Config, somehow};

use super::{Base, Tab};

#[derive(Template)]
#[template(path = "queue.html")]
struct CommitTemplate {
    base: Base,
}

pub async fn get(State(config): State<&'static Config>) -> somehow::Result<impl IntoResponse> {
    Ok(CommitTemplate {
        base: Base::new(config, Tab::Queue),
    })
}
