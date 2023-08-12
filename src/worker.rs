mod run;
mod server;
mod tree;

use std::sync::{Arc, Mutex};

use reqwest::Client;
use time::OffsetDateTime;
use tokio::sync::Mutex as AsyncMutex;
use tracing::{error, info, warn};

use crate::{
    config::Config,
    id,
    shared::{FinishedRun, Run},
    worker::server::Server,
};

use self::run::RunInProgress;

pub struct Worker {
    config: &'static Config,
}

impl Worker {
    pub fn new(config: &'static Config) -> Self {
        Self { config }
    }

    pub async fn run(&self) {
        let client = Client::new();
        let current_run = Arc::new(Mutex::new(None));

        let mut servers = self
            .config
            .worker_servers
            .iter()
            .map(|(name, server_config)| Server {
                name: name.clone(),
                config: self.config,
                server_config,
                secret: id::random_worker_secret(),
                client: client.clone(),
                current_run: current_run.clone(),
                status_lock: Arc::new(AsyncMutex::new(())),
            })
            .collect::<Vec<_>>();

        for server in &servers {
            info!("Connecting to server {}", server.name);
            tokio::spawn(server.clone().ping_periodically());
        }

        match servers.len() {
            0 => error!("No servers specified in config"),
            1 => self.single_server_mode(servers.pop().unwrap()).await,
            _ => self.many_server_mode(servers).await,
        }
    }

    async fn single_server_mode(&self, server: Server) {
        loop {
            while self.perform_run(&server).await {}
            tokio::time::sleep(self.config.worker_ping_delay).await;
        }
    }

    async fn many_server_mode(&self, servers: Vec<Server>) {
        loop {
            for server in &servers {
                let batch_start = OffsetDateTime::now_utc();
                let batch_end = batch_start + self.config.worker_batch_duration;
                while OffsetDateTime::now_utc() <= batch_end {
                    if !self.perform_run(server).await {
                        break;
                    }
                }
            }
            tokio::time::sleep(self.config.worker_ping_delay).await;
        }
    }

    /// Ask a server for a run, do the run, send results to the server.
    ///
    /// Returns whether a run was performed.
    async fn perform_run(&self, server: &Server) -> bool {
        // Request run
        let guard = server.status_lock.lock().await;
        let Some(run) = self.request_run(server).await else { return false; };
        let run = RunInProgress::new(server.name.clone(), server.server_config, run);
        *server.current_run.lock().unwrap() = Some(run.clone());
        drop(guard);

        // Perform run
        let Some(run) = run.perform(server).await else { return false; };

        // Submit run
        let guard = server.status_lock.lock().await;
        *server.current_run.lock().unwrap() = None;
        while !self.submit_run(server, run.clone()).await {
            tokio::time::sleep(self.config.worker_ping_delay).await;
        }
        drop(guard);

        true
    }

    async fn request_run(&self, server: &Server) -> Option<Run> {
        match server.post_status(true, None).await {
            Ok(response) => response.run,
            Err(e) => {
                warn!("Error requesting run:\n{e:?}");
                None
            }
        }
    }

    async fn submit_run(&self, server: &Server, run: FinishedRun) -> bool {
        match server.post_status(false, Some(run)).await {
            Ok(_) => true,
            Err(e) => {
                warn!("Error submitting run:\n{e:?}");
                false
            }
        }
    }
}
