//! Recurring actions and updates.

// TODO `fetch` submodule for fetching new commits
// TODO `queue` submodule for updating the queue

mod queue;
mod repo;

use tracing::{debug_span, error, Instrument};

use super::Server;

async fn recurring_task(state: &Server) {
    async {
        if let Err(e) = repo::update(&state.db, state.repo.clone()).await {
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

pub async fn run(state: Server) {
    loop {
        recurring_task(&state).await;
        tokio::time::sleep(state.config.repo.update_delay).await;
    }
}
