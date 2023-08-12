mod server;
mod tree;

use reqwest::Client;
use tracing::error;

use crate::{config::Config, worker::server::Server};

pub struct Worker {
    config: &'static Config,
}

impl Worker {
    pub fn new(config: &'static Config) -> Self {
        Self { config }
    }

    pub async fn run(&self) {
        let client = Client::new();

        let mut servers = self
            .config
            .worker_servers
            .iter()
            .map(|(name, server_config)| {
                Server::new(name.clone(), self.config, server_config, client.clone())
            })
            .collect::<Vec<_>>();

        for server in &servers {
            tokio::spawn(server.clone().ping_periodically());
        }

        match servers.len() {
            0 => error!("No servers specified in config"),
            1 => self.single_server_mode(servers.pop().unwrap()).await,
            _ => self.many_server_mode(servers).await,
        }
    }

    async fn single_server_mode(&self, server: Server) {
        // TODO Implement
    }

    async fn many_server_mode(&self, servers: Vec<Server>) {
        // TODO Implement
    }
}
