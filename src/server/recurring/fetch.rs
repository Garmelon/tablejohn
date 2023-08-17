//! Update repo refs using the `git` binary.

use log::{info, warn};

use crate::{config::ServerConfig, git, server::Repo, somehow};

async fn inner(repo: Repo, url: &'static str, refspecs: &'static [String]) -> somehow::Result<()> {
    tokio::task::spawn_blocking(move || git::fetch(repo.0.path(), url, refspecs)).await??;
    Ok(())
}

pub(super) async fn update(config: &'static ServerConfig, repo: Repo) {
    if let Some(url) = &config.repo_fetch_url {
        info!("Fetching refs from {url}");
        if let Err(e) = inner(repo, url, &config.repo_fetch_refspecs).await {
            warn!("Error fetching refs:\n{e:?}");
        }
    }
}
