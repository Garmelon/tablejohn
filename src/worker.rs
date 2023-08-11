mod coordinator;
mod server;
mod tree;

use std::sync::{Arc, Mutex};

use tokio::task::JoinSet;
use tracing::{debug, error};

use crate::config::{Config, WorkerServerConfig};

use self::{coordinator::Coordinator, server::Server};

pub struct Worker {
    config: &'static Config,
}

impl Worker {
    pub fn new(config: &'static Config) -> Self {
        Self { config }
    }

    pub async fn run(&self) {
        if self.config.worker_servers.is_empty() {
            error!("No servers specified in config");
            return;
        }

        let coordinator = Arc::new(Mutex::new(Coordinator::new()));

        let mut tasks = JoinSet::new();
        for (name, server_config) in self.config.worker_servers.iter() {
            debug!("Launching task for server {name}");
            let mut server = Server::new(
                name.clone(),
                self.config,
                server_config,
                coordinator.clone(),
            );
            tasks.spawn(async move { server.run().await });
        }

        while tasks.join_next().await.is_some() {}
    }
}

pub fn launch_standalone_server_task(
    config: &'static Config,
    server_name: String,
    server_config: &'static WorkerServerConfig,
) {
    let coordinator = Arc::new(Mutex::new(Coordinator::new()));
    let mut server = Server::new(server_name, config, server_config, coordinator);
    tokio::task::spawn(async move { server.run().await });
}
