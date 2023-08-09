mod coordinator;
mod server;

use std::sync::{Arc, Mutex};

use tokio::task::JoinSet;
use tracing::{debug, error};

use crate::config::Config;

use self::{coordinator::Coordinator, server::Server};

pub struct Runner {
    config: &'static Config,
}

impl Runner {
    pub fn new(config: &'static Config) -> Self {
        Self { config }
    }

    pub async fn run(&self) {
        if self.config.runner_servers.is_empty() {
            error!("No servers specified in config");
            return;
        }

        let names = self.config.runner_servers.keys().cloned().collect();
        let coordinator = Arc::new(Mutex::new(Coordinator::new(names)));

        let mut tasks = JoinSet::new();
        for (name, config) in self.config.runner_servers.iter() {
            debug!("Launching task for server {name}");
            let mut server = Server::new(
                name.clone(),
                config,
                self.config.runner_ping_delay,
                coordinator.clone(),
            );
            tasks.spawn(async move { server.run().await });
        }

        while tasks.join_next().await.is_some() {}
    }
}
