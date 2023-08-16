//! Recurring actions and updates.

// TODO `fetch` submodule for fetching new commits
// TODO `queue` submodule for updating the queue

mod queue;
mod repo;

use tracing::{debug_span, error, Instrument};

use super::{Repo, Server};

async fn recurring_task(state: &Server, repo: Repo) {
    async {
        if let Err(e) = repo::update(&state.db, repo).await {
            error!("Error updating repo:\n{e:?}");
        };
    }
    .instrument(debug_span!("update repo"))
    .await;

    async {
        if let Err(e) = queue::update(&state.db).await {
            error!("Error updating queue:\n{e:?}");
        };
    }
    .instrument(debug_span!("update queue"))
    .await;
}

pub(super) async fn run(server: Server, repo: Repo) {
    loop {
        recurring_task(&server, repo.clone()).await;
        tokio::time::sleep(server.config.repo_update).await;
    }
}
