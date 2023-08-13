mod admin;
mod api;
mod base;
mod index;
mod link;
mod pages;
pub mod paths;
mod queue;
mod r#static;
mod worker;

use axum::{routing::get, Router};
use axum_extra::routing::RouterExt;

use crate::somehow;

use self::{
    admin::queue::post_admin_queue_add,
    api::worker::{
        get_api_worker_bench_repo_by_hash_tree_tar_gz, get_api_worker_repo_by_hash_tree_tar_gz,
        post_api_worker_status,
    },
    index::get_index,
    pages::{commit::get_commit_by_hash, run::get_run_by_id},
    queue::{get_queue, get_queue_inner},
    worker::get_worker_by_name,
};

use super::Server;

pub async fn run(server: Server) -> somehow::Result<()> {
    // TODO Add text body to body-less status codes

    let app = Router::new()
        .typed_get(get_api_worker_bench_repo_by_hash_tree_tar_gz)
        .typed_get(get_api_worker_repo_by_hash_tree_tar_gz)
        .typed_get(get_commit_by_hash)
        .typed_get(get_index)
        .typed_get(get_queue)
        .typed_get(get_queue_inner)
        .typed_get(get_run_by_id)
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
