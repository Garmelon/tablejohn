//! Recurring actions and updates.

// TODO `fetch` submodule for fetching new commits
// TODO `queue` submodule for updating the queue

mod repo;

use tracing::{debug_span, error, Instrument};

use crate::state::AppState;

async fn recurring_task(state: &AppState) {
    async {
        if let Err(e) = repo::update(&state.db, state.repo.clone()).await {
            error!("Error updating repo:\n{e:?}");
        };
    }
    .instrument(debug_span!("update repo"))
    .await;
}

pub async fn run(state: AppState) {
    loop {
        recurring_task(&state).await;
        tokio::time::sleep(state.config.repo.update_delay).await;
    }
}
