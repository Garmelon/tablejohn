use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use time::OffsetDateTime;
use tracing::{debug, info_span, warn, Instrument};

use crate::{config::RunnerServerConfig, somehow};

use super::coordinator::Coordinator;

enum RunState {
    Preparing,
    Running,
    Finished, // TODO Include run results here
}

struct Run {
    id: String,
    hash: String,
    start: OffsetDateTime,
    state: RunState,
}

pub struct Server {
    name: String,
    config: &'static RunnerServerConfig,
    ping_delay: Duration,
    coordinator: Arc<Mutex<Coordinator>>,
    run: Option<Arc<Mutex<Run>>>,
}

impl Server {
    pub fn new(
        name: String,
        config: &'static RunnerServerConfig,
        ping_delay: Duration,
        coordinator: Arc<Mutex<Coordinator>>,
    ) -> Self {
        Self {
            name,
            config,
            ping_delay,
            coordinator,
            run: None,
        }
    }

    pub async fn run(&mut self) {
        let name = self.name.clone();
        async {
            loop {
                match self.ping().await {
                    Ok(()) => {}
                    Err(e) => warn!("Error talking to server:\n{e:?}"),
                }
                tokio::time::sleep(self.ping_delay).await;
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
