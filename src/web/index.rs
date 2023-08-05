use askama::Template;
use axum::{extract::State, response::IntoResponse};

use crate::config::Config;

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    base: String,
    repo_name: String,
}

pub async fn get(State(config): State<&'static Config>) -> super::Result<impl IntoResponse> {
    Ok(IndexTemplate {
        base: config.web.base(),
        repo_name: config.repo.name(),
    })
}
