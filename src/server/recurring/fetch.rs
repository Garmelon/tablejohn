//! Update repo refs using the `git` binary.

use std::process::Command;

use gix::bstr::ByteSlice;
use log::{info, warn};

use crate::{config::ServerConfig, server::Repo, somehow};

fn fetch(repo: Repo, url: &str, refspecs: &[String]) -> somehow::Result<()> {
    info!("Fetching refs from {url}");

    let mut command = Command::new("git");
    command
        .arg("fetch")
        .arg("-C")
        .arg(repo.0.path())
        .arg("--prune")
        .arg("--")
        .arg(url);
    for refspec in refspecs {
        command.arg(refspec);
    }

    let output = command.output()?;
    if output.status.success() {
    } else {
        warn!(
            "Error fetching refs:\n\
            {command:?} exited with code {}\n\
            STDOUT:\n{}\n\
            STDERR:\n{}",
            output.status,
            output.stdout.to_str_lossy(),
            output.stderr.to_str_lossy()
        );
    }

    Ok(())
}

async fn inner(repo: Repo, url: &'static str, refspecs: &'static [String]) -> somehow::Result<()> {
    tokio::task::spawn_blocking(move || fetch(repo, url, refspecs)).await??;
    Ok(())
}

pub(super) async fn update(config: &'static ServerConfig, repo: Repo) {
    if let Some(url) = &config.repo_fetch_url {
        if let Err(e) = inner(repo, url, &config.repo_fetch_refspecs).await {
            warn!("Error fetching refs:\n{e:?}");
        }
    }
}
