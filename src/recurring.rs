//! Recurring actions and updates.

// TODO `fetch` submodule for fetching new commits
// TODO `queue` submodule for updating the queue

mod repo;

use tracing::{debug_span, error, Instrument};

use crate::state::AppState;

async fn recurring_task(state: &AppState) {
    let repo = state.repo.to_thread_local();

    async {
        if let Err(e) = repo::update(&state.db, &repo).await {
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
