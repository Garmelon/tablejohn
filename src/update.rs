//! Repeatedly update the db from the repo.

use tracing::{warn, debug};

use crate::state::AppState;

async fn update_repo(state: &AppState) -> anyhow::Result<()> {
    debug!("Updating repo");
    Ok(())
}

pub async fn repeatedly(state: AppState) {
    loop {
        if let Err(e) = update_repo(&state).await {
            warn!("Error while updating repo: {e:?}");
        }
        tokio::time::sleep(state.config.repo_update_delay).await;
    }
}
