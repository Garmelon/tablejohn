use reqwest::Client;
use tempfile::TempDir;
use tracing::{debug, warn};

use crate::{
    config::{Config, WorkerServerConfig},
    id,
    shared::{FinishedRun, ServerResponse, WorkerRequest, WorkerStatus},
    somehow,
    worker::tree,
};

#[derive(Clone)]
pub struct Server {
    name: String,
    config: &'static Config,
    server_config: &'static WorkerServerConfig,
    client: Client,
    secret: String,
}

impl Server {
    pub fn new(
        name: String,
        config: &'static Config,
        server_config: &'static WorkerServerConfig,
        client: Client,
    ) -> Self {
        Self {
            name,
            config,
            server_config,
            client,
            secret: id::random_worker_secret(),
        }
    }

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
            request_work,
            submit_work,
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

        // TODO Use actual status
        let status = WorkerStatus::Idle;

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
