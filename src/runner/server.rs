use std::sync::{Arc, Mutex};

use tokio::sync::mpsc;
use tracing::{debug, info_span, warn, Instrument};

use crate::{
    config::{Config, RunnerServerConfig},
    somehow,
};

use super::coordinator::Coordinator;

pub struct Server {
    name: String,
    config: &'static Config,
    server_config: &'static RunnerServerConfig,
    coordinator: Arc<Mutex<Coordinator>>,
}

impl Server {
    pub fn new(
        name: String,
        config: &'static Config,
        server_config: &'static RunnerServerConfig,
        coordinator: Arc<Mutex<Coordinator>>,
    ) -> Self {
        Self {
            name,
            config,
            server_config,
            coordinator,
        }
    }

    pub async fn run(&mut self) {
        let (poke_tx, mut poke_rx) = mpsc::unbounded_channel();
        self.coordinator
            .lock()
            .unwrap()
            .register(self.name.clone(), poke_tx.clone());

        let name = self.name.clone();
        async {
            loop {
                match self.ping().await {
                    Ok(()) => {}
                    Err(e) => warn!("Error talking to server:\n{e:?}"),
                }

                // Wait for poke or until the ping delay elapses. If we get
                // poked while pinging the server, this will not wait and we'll
                // immediately do another ping.
                let _ = tokio::time::timeout(self.config.runner_ping_delay, poke_rx.recv()).await;

                // Empty queue in case we were poked more than once. This can
                // happen for example if we get poked multiple times while
                // pinging the server.
                while poke_rx.try_recv().is_ok() {}
            }
        }
        .instrument(info_span!("runner", name))
        .await;
    }

    async fn ping(&mut self) -> somehow::Result<()> {
        debug!("Pinging");
        Ok(())
    }
}
