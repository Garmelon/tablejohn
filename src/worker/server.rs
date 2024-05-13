use std::sync::{Arc, Mutex};

use log::{debug, warn};
use reqwest::Client;
use tempfile::TempDir;
use tokio::sync::Mutex as AsyncMutex;

use crate::{
    config::{WorkerConfig, WorkerServerConfig},
    server::web::paths::{
        PathApiWorkerBenchRepoByHashTreeTarGz, PathApiWorkerRepoByHashTreeTarGz,
        PathApiWorkerStatus,
    },
    shared::{FinishedRun, ServerResponse, WorkerRequest, WorkerStatus},
    somehow,
    worker::tree,
};

use super::run::RunInProgress;

#[derive(Clone)]
pub struct Server {
    pub name: String,
    pub config: &'static WorkerConfig,
    pub server_config: &'static WorkerServerConfig,
    pub secret: String,

    pub client: Client,
    pub current_run: Arc<Mutex<Option<RunInProgress>>>,

    /// You must hold this lock while sending status updates to the server and
    /// while processing the response.
    ///
    /// This lock prevents the following race condition that would lead to
    /// multiple workers receiving runs for the same commit in unlucky
    /// circumstances:
    ///
    /// 1. The main task requests a run
    /// 2. The ping task sends a status update where the worker is idle
    /// 3. The server receives 1, reserves a run and replies
    /// 4. The server receives 2 and clears the reservation
    /// 5. Another worker requests a run before this worker's next ping
    pub status_lock: Arc<AsyncMutex<()>>,
}

impl Server {
    /// **Important:** Before using this function, read the documentation of
    /// [`Self::status_lock`]!
    pub async fn post_status(
        &self,
        request_run: bool,
        submit_run: Option<FinishedRun>,
    ) -> somehow::Result<ServerResponse> {
        let url = format!("{}{}", self.server_config.url, PathApiWorkerStatus {});

        let status = match &*self.current_run.lock().unwrap() {
            Some(run) if run.is_for_server(&self.name) => {
                WorkerStatus::Working(run.as_unfinished_run())
            }
            Some(_) => WorkerStatus::Busy,
            None => WorkerStatus::Idle,
        };

        let request = WorkerRequest {
            info: None,
            secret: self.secret.clone(),
            status,
            request_run,
            submit_run,
        };

        let response = self
            .client
            .post(url)
            .basic_auth(&self.config.name, Some(&self.server_config.token))
            .json(&request)
            .send()
            .await?
            .error_for_status()?
            .json::<ServerResponse>()
            .await?;

        if response.abort_run {
            if let Some(current_run) = &*self.current_run.lock().unwrap() {
                current_run.abort();
            }
        }

        Ok(response)
    }

    pub async fn download_repo(&self, hash: &str) -> somehow::Result<TempDir> {
        let url = format!(
            "{}{}",
            self.server_config.url,
            PathApiWorkerRepoByHashTreeTarGz {
                hash: hash.to_string()
            },
        );

        let response = self
            .client
            .get(url)
            .basic_auth(&self.config.name, Some(&self.server_config.token))
            .send()
            .await?;

        tree::download(response).await
    }

    pub async fn download_bench_repo(&self, hash: &str) -> somehow::Result<TempDir> {
        let url = format!(
            "{}{}",
            self.server_config.url,
            PathApiWorkerBenchRepoByHashTreeTarGz {
                hash: hash.to_string()
            },
        );

        let response = self
            .client
            .get(url)
            .basic_auth(&self.config.name, Some(&self.server_config.token))
            .send()
            .await?;

        tree::download(response).await
    }

    async fn ping(&self) -> somehow::Result<()> {
        debug!("Pinging {}", self.name);

        let guard = self.status_lock.lock().await;
        let _ = self.post_status(false, None).await?;
        drop(guard);

        Ok(())
    }

    pub async fn ping_periodically(self) {
        loop {
            match self.ping().await {
                Ok(()) => {}
                Err(e) => warn!("Error talking to {}:\n{e:?}", self.name),
            }

            tokio::time::sleep(self.config.ping).await;
        }
    }
}
