mod commit;
mod commit_hash;
mod index;
mod queue;
mod r#static;

use axum::{routing::get, Router};

use crate::{config::Config, somehow};

use super::Server;

pub enum Tab {
    Index,
    Commit,
    Queue,
}

#[derive(Clone)]
pub struct Base {
    root: String,
    repo_name: String,
    current: String,
}

impl Base {
    pub fn new(config: &Config, tab: Tab) -> Self {
        let current = match tab {
            Tab::Index => "index",
            Tab::Commit => "commit",
            Tab::Queue => "queue",
        };
        Self {
            root: config.web_base.clone(),
            repo_name: config.repo_name.clone(),
            current: current.to_string(),
        }
    }
}

pub async fn run(server: Server) -> somehow::Result<()> {
    // TODO Add text body to body-less status codes

    let app = Router::new()
        .route("/", get(index::get))
        .route("/commit/", get(commit::get))
        .route("/commit/:hash", get(commit_hash::get))
        .route("/queue/", get(queue::get))
        .route("/queue/table", get(queue::get_table))
        .fallback(get(r#static::static_handler))
        .with_state(server.clone());

    axum::Server::bind(&server.config.web_address)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
