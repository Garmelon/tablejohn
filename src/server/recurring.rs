//! Recurring actions and updates.

mod fetch;
mod queue;
mod repo;

use tokio::sync::mpsc;

use super::{Repo, Server};

pub(super) async fn run(server: Server, repo: Repo, mut recurring_rx: mpsc::UnboundedReceiver<()>) {
    loop {
        fetch::update(server.config, repo.clone()).await;
        repo::update(&server.db, repo.clone()).await;
        queue::update(&server.db).await;

        let _ = tokio::time::timeout(server.config.repo_update, recurring_rx.recv()).await;
        while let Ok(()) = recurring_rx.try_recv() {}
    }
}
