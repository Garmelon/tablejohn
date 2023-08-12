use std::sync::{Arc, Mutex};

use reqwest::Client;
use tempfile::TempDir;
use tracing::{debug, warn};

use crate::{
    config::{Config, WorkerServerConfig},
    shared::{FinishedRun, ServerResponse, UnfinishedRun, WorkerRequest, WorkerStatus},
    somehow,
    worker::tree,
};

use super::run::RunInProgress;

const SCROLLBACK: usize = 50;

#[derive(Clone)]
pub struct Server {
    pub name: String,
    pub config: &'static Config,
    pub server_config: &'static WorkerServerConfig,
    pub secret: String,
    pub client: Client,
    pub current_run: Arc<Mutex<Option<RunInProgress>>>,
}

impl Server {
    // TODO Limit status requests to one in flight at a time (per server)
    pub async fn post_status(
        &self,
        status: WorkerStatus,
        request_work: bool,
        submit_work: Option<FinishedRun>,
    ) -> somehow::Result<ServerResponse> {
        let url = format!("{}api/worker/status", self.server_config.url);
        let request = WorkerRequest {
            info: None,
            secret: self.secret.clone(),
            status,
            request_run: request_work,
            submit_run: submit_work,
        };

        let response = self
            .client
            .post(url)
            .basic_auth(&self.config.worker_name, Some(&self.server_config.token))
            .json(&request)
            .send()
            .await?
            .json::<ServerResponse>()
            .await?;

        Ok(response)
    }

    pub async fn download_repo(&self, hash: &str) -> somehow::Result<TempDir> {
        let url = format!(
            "{}api/worker/repo/{hash}/tree.tar.gz",
            self.server_config.url
        );

        let response = self
            .client
            .get(url)
            .basic_auth(&self.config.worker_name, Some(&self.server_config.token))
            .send()
            .await?;

        tree::download(response).await
    }

    pub async fn download_bench_repo(&self, hash: &str) -> somehow::Result<TempDir> {
        let url = format!(
            "{}api/worker/bench_repo/{hash}/tree.tar.gz",
            self.server_config.url
        );

        let response = self
            .client
            .get(url)
            .basic_auth(&self.config.worker_name, Some(&self.server_config.token))
            .send()
            .await?;

        tree::download(response).await
    }

    async fn ping(&self) -> somehow::Result<()> {
        debug!("Pinging server");

        let status = match &*self.current_run.lock().unwrap() {
            Some(run) if run.server_name == self.name => WorkerStatus::Working(UnfinishedRun {
                run: run.run.clone(),
                last_output: run
                    .output
                    .lock()
                    .unwrap()
                    .iter()
                    .rev()
                    .take(SCROLLBACK)
                    .rev()
                    .cloned()
                    .collect(),
            }),
            Some(_) => WorkerStatus::Busy,
            None => WorkerStatus::Idle,
        };

        let response = self.post_status(status, false, None).await?;

        // TODO Signal that run should be aborted

        Ok(())
    }

    pub async fn ping_periodically(self) {
        loop {
            match self.ping().await {
                Ok(()) => {}
                Err(e) => warn!("Error talking to server:\n{e:?}"),
            }

            tokio::time::sleep(self.config.worker_ping_delay).await;
        }
    }
}
