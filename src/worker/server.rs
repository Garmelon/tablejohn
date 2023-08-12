use std::sync::{Arc, Mutex};

use reqwest::Client;
use tokio::sync::mpsc;
use tracing::{debug, info_span, warn, Instrument};

use crate::{
    config::{Config, WorkerServerConfig},
    id,
    shared::{FinishedRun, ServerResponse, WorkerRequest, WorkerStatus},
    somehow,
    worker::run::{self, FullRunStatus},
};

use super::{
    coordinator::{ActiveInfo, Coordinator},
    run::Run,
};

pub struct Server {
    name: String,
    config: &'static Config,
    server_config: &'static WorkerServerConfig,
    coordinator: Arc<Mutex<Coordinator>>,
    client: Client,
    secret: String,

    // TODO Cache bench dir
    run: Option<(Arc<Mutex<Run>>, mpsc::UnboundedSender<()>)>,
}

impl Server {
    pub fn new(
        name: String,
        config: &'static Config,
        server_config: &'static WorkerServerConfig,
        coordinator: Arc<Mutex<Coordinator>>,
    ) -> Self {
        Self {
            name,
            config,
            server_config,
            coordinator,
            client: Client::new(),
            secret: id::random_worker_secret(),
            run: None,
        }
    }

    pub async fn run(&mut self) {
        // Register with coordinator
        let (poke_tx, mut poke_rx) = mpsc::unbounded_channel();
        self.coordinator
            .lock()
            .unwrap()
            .register(self.name.clone(), poke_tx.clone());

        // Main loop
        let name = self.name.clone();
        async {
            loop {
                match self.ping().await {
                    Ok(()) => {}
                    Err(e) => warn!("Error talking to server:\n{e:?}"),
                }

                self.wait_until_next_ping(&mut poke_rx).await;
            }
        }
        .instrument(info_span!("worker", name))
        .await;
    }

    async fn wait_until_next_ping(&self, poke_rx: &mut mpsc::UnboundedReceiver<()>) {
        // Wait for poke or until the ping delay elapses. If we get poked while
        // pinging the server, this will not wait and we'll immediately do
        // another ping.
        let _ = tokio::time::timeout(self.config.worker_ping_delay, poke_rx.recv()).await;

        // Empty queue in case we were poked more than once. This can happen for
        // example if we get poked multiple times while pinging the server.
        while poke_rx.try_recv().is_ok() {}
    }

    async fn ping(&mut self) -> somehow::Result<()> {
        debug!("Pinging server");

        let info = self.coordinator.lock().unwrap().active(&self.name);
        if info.active {
            self.ping_active(info).await?;
        } else {
            self.ping_inactive(info).await?;
        }

        Ok(())
    }

    async fn ping_inactive(&self, info: ActiveInfo) -> somehow::Result<()> {
        assert!(self.run.is_none());

        let status = match info.busy {
            true => WorkerStatus::Busy,
            false => WorkerStatus::Idle,
        };
        self.request(status, false, None).await?;
        Ok(())
    }

    async fn ping_active(&mut self, info: ActiveInfo) -> somehow::Result<()> {
        let run = self
            .run
            .as_ref()
            .map(|(r, _)| r.lock().unwrap().clone().into_full_status())
            .unwrap_or(FullRunStatus::Aborted);

        let unfinished = matches!(run, FullRunStatus::Unfinished(_));
        let aborted = matches!(run, FullRunStatus::Aborted);
        let in_batch = info.in_batch(self.config.worker_batch_duration);

        let (status, submit_work) = match run {
            FullRunStatus::Unfinished(run) => (WorkerStatus::Working(run), None),
            FullRunStatus::Finished(run) => (WorkerStatus::Idle, Some(run)),
            FullRunStatus::Aborted => (WorkerStatus::Idle, None),
        };
        let request_work = in_batch && !unfinished;
        let response = self.request(status, request_work, submit_work).await;

        if response.is_err() && aborted {
            // We have nothing important going on, let's defer to the next
            // server and hope this one will respond again soon.
            self.coordinator
                .lock()
                .unwrap()
                .move_to_next_server(&self.name);

            // Return explicitly to ensure we don't continue to the rest of the
            // function in the false belief that we're active. Oh, and don't
            // swallow the error.
            response?;
            return Ok(());
        }

        let response = response?;

        // Clean up self.run if we no longer need it
        if !unfinished {
            // We can get rid of finished runs since we just successfully sent
            // the server the results.
            self.run = None;
        }

        // Abort run if server says so
        if response.abort_work {
            if let Some((_, abort_tx)) = &self.run {
                let _ = abort_tx.send(());
            }
        }

        // Start work (but only if we requested it)
        if let Some(work) = response.work.filter(|_| request_work) {
            assert!(!unfinished);
            assert!(self.run.is_none());

            let run = Arc::new(Mutex::new(Run::new(work.hash)));
            let (abort_tx, abort_rx) = mpsc::unbounded_channel();

            self.run = Some((run.clone(), abort_tx));
            self.coordinator.lock().unwrap().look_busy(&self.name);
            tokio::spawn(run::run(run, abort_rx, work.bench));
        }

        // Finally, advance to the next server if it makes sense to do so
        if self.run.is_none() {
            self.coordinator
                .lock()
                .unwrap()
                .move_to_next_server(&self.name);
        }

        Ok(())
    }

    async fn request(
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
}
