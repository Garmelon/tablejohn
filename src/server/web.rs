mod admin;
mod api;
mod commit;
mod index;
mod link;
pub mod paths;
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
    commit::get_commit_by_hash,
    index::get_index,
    queue::{get_queue, get_queue_inner},
    worker::get_worker_by_name,
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
        .typed_get(get_api_worker_bench_repo_by_hash_tree_tar_gz)
        .typed_get(get_api_worker_repo_by_hash_tree_tar_gz)
        .typed_get(get_commit_by_hash)
        .typed_get(get_index)
        .typed_get(get_queue)
        .typed_get(get_queue_inner)
        .typed_get(get_worker_by_name)
        .typed_post(post_admin_queue_add)
        .typed_post(post_api_worker_status)
        .fallback(get(r#static::static_handler))
        .with_state(server.clone());

    axum::Server::bind(&server.config.web_address)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
