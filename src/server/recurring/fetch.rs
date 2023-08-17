//! Update repo refs using the `git` binary.

use log::{debug, info, warn};

use crate::{config::ServerConfig, git, server::Repo, somehow};

async fn inner(repo: Repo, url: &'static str, refspecs: &'static [String]) -> somehow::Result<()> {
    let output =
        tokio::task::spawn_blocking(move || git::fetch(repo.0.path(), url, refspecs)).await??;
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.is_empty() {
        info!("Fetched refs:\n{}", stderr.trim_end());
    }
    Ok(())
}

pub(super) async fn update(config: &'static ServerConfig, repo: Repo) {
    if let Some(url) = &config.repo_fetch_url {
        debug!("Fetching refs from {url}");
        if let Err(e) = inner(repo, url, &config.repo_fetch_refspecs).await {
            warn!("Error fetching refs:\n{e:?}");
        }
    }
}
