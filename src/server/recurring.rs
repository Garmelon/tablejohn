//! Recurring actions and updates.

mod fetch;
mod queue;
mod repo;

use super::{Repo, Server};

pub(super) async fn run(server: Server, repo: Repo) {
    loop {
        fetch::update(server.config, repo.clone()).await;
        repo::update(&server.db, repo.clone()).await;
        queue::update(&server.db).await;

        tokio::time::sleep(server.config.repo_update).await;
    }
}
