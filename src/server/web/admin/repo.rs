use std::sync::Arc;

use axum::{
    extract::State,
    response::{IntoResponse, Redirect},
};
use log::info;
use tokio::sync::mpsc;

use crate::{
    config::ServerConfig,
    server::web::{
        base::Base,
        paths::{PathAdminRepoUpdate, PathIndex},
    },
    somehow,
};

pub async fn post_admin_repo_update(
    _path: PathAdminRepoUpdate,
    State(config): State<&'static ServerConfig>,
    State(recurring_tx): State<Arc<mpsc::UnboundedSender<()>>>,
) -> somehow::Result<impl IntoResponse> {
    let _ = recurring_tx.send(());
    info!("Admin updated repo");

    let link = Base::link_with_config(config, PathIndex {});
    Ok(Redirect::to(&link.to_string()))
}
