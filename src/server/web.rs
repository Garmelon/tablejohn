mod api;
mod commit;
mod index;
mod link;
mod queue;
mod runner;
mod r#static;

use axum::{routing::get, Router};

use crate::{config::Config, somehow};

use super::Server;

pub enum Tab {
    None,
    Index,
    Queue,
}

#[derive(Clone)]
pub struct Base {
    root: String,
    repo_name: String,
    current: &'static str,
}

impl Base {
    pub fn new(config: &Config, tab: Tab) -> Self {
        let current = match tab {
            Tab::None => "",
            Tab::Index => "index",
            Tab::Queue => "queue",
        };
        Self {
            root: config.web_base.clone(),
            repo_name: config.repo_name.clone(),
            current,
        }
    }
}

pub async fn run(server: Server) -> somehow::Result<()> {
    // TODO Add text body to body-less status codes

    let app = Router::new()
        .route("/", get(index::get))
        .route("/commit/:hash", get(commit::get))
        .route("/runner/:name", get(runner::get))
        .route("/queue/", get(queue::get))
        .route("/queue/table", get(queue::get_table))
        .merge(api::router(&server))
        .fallback(get(r#static::static_handler))
        .with_state(server.clone());

    axum::Server::bind(&server.config.web_address)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
