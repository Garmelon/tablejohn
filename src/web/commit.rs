use askama::Template;
use axum::{extract::State, response::IntoResponse};

use crate::{config::Config, somehow};

#[derive(Template)]
#[template(path = "commit.html")]
struct CommitTemplate {
    base: String,
    repo_name: String,
    current: String,
}

pub async fn get(State(config): State<&'static Config>) -> somehow::Result<impl IntoResponse> {
    Ok(CommitTemplate {
        base: config.web.base(),
        repo_name: config.repo.name(),
        current: "commit".to_string(),
    })
}
