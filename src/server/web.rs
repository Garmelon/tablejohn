mod admin;
pub mod api;
mod commit;
mod index;
mod link;
mod queue;
mod r#static;
mod worker;

use axum::{routing::get, Router};
use axum_extra::routing::RouterExt;

use crate::{config::Config, somehow};

use self::{
    admin::queue::post_admin_queue_add,
    api::worker::{
        get_api_worker_bench_repo_by_hash_tree_tar_gz, get_api_worker_repo_by_hash_tree_tar_gz,
        post_api_worker_status,
    },
};

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
        .route("/queue/", get(queue::get))
        .route("/queue/inner", get(queue::get_inner))
        .route("/worker/:name", get(worker::get))
        .typed_get(get_api_worker_bench_repo_by_hash_tree_tar_gz)
        .typed_get(get_api_worker_repo_by_hash_tree_tar_gz)
        .typed_post(post_admin_queue_add)
        .typed_post(post_api_worker_status)
        .fallback(get(r#static::static_handler))
        .with_state(server.clone());

    axum::Server::bind(&server.config.web_address)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
