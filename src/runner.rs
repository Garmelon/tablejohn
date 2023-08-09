mod coordinator;

use tracing::error;

use crate::config::Config;

pub struct Runner {
    config: &'static Config,
}

impl Runner {
    pub fn new(config: &'static Config) -> Self {
        Self { config }
    }

    pub async fn run(&self) {
        error!("Runner not yet implemented");
    }
}
