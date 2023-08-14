mod admin;
mod api;
mod base;
mod link;
mod pages;
pub mod paths;
mod r#static;

use axum::{extract::DefaultBodyLimit, routing::get, Router};
use axum_extra::routing::RouterExt;

use crate::somehow;

use self::{
    admin::queue::{
        post_admin_queue_add, post_admin_queue_decrease, post_admin_queue_delete,
        post_admin_queue_increase,
    },
    api::worker::{
        get_api_worker_bench_repo_by_hash_tree_tar_gz, get_api_worker_repo_by_hash_tree_tar_gz,
        post_api_worker_status,
    },
    pages::{
        commit::get_commit_by_hash,
        graph::{get_graph, get_graph_data},
        index::get_index,
        queue::{get_queue, get_queue_delete, get_queue_inner},
        run::get_run_by_id,
        worker::get_worker_by_name,
    },
};

use super::Server;

pub async fn run(server: Server) -> somehow::Result<()> {
    // TODO Add text body to body-less status codes

    let post_api_worker_status = Router::new()
        .typed_post(post_api_worker_status)
        .layer(DefaultBodyLimit::max(server.config.web_worker_max_upload));

    let app = Router::new()
        .typed_get(get_api_worker_bench_repo_by_hash_tree_tar_gz)
        .typed_get(get_api_worker_repo_by_hash_tree_tar_gz)
        .typed_get(get_commit_by_hash)
        .typed_get(get_graph)
        .typed_get(get_graph_data)
        .typed_get(get_index)
        .typed_get(get_queue)
        .typed_get(get_queue_delete)
        .typed_get(get_queue_inner)
        .typed_get(get_run_by_id)
        .typed_get(get_worker_by_name)
        .typed_post(post_admin_queue_add)
        .typed_post(post_admin_queue_decrease)
        .typed_post(post_admin_queue_delete)
        .typed_post(post_admin_queue_increase)
        .merge(post_api_worker_status)
        .fallback(get(r#static::static_handler))
        .with_state(server.clone());

    axum::Server::bind(&server.config.web_address)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
