mod tree;

use tracing::error;

use crate::config::Config;

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

        todo!()
    }
}
