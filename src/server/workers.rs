use std::collections::HashMap;

use gix::hashtable::HashSet;
use time::OffsetDateTime;

use crate::{config::Config, shared::WorkerStatus};

#[derive(Clone)]
pub struct WorkerInfo {
    pub secret: String,
    pub last_seen: OffsetDateTime,
    pub status: WorkerStatus,
}

impl WorkerInfo {
    pub fn new(secret: String, last_seen: OffsetDateTime, status: WorkerStatus) -> Self {
        Self {
            secret,
            last_seen,
            status,
        }
    }
}

pub struct Workers {
    config: &'static Config,
    workers: HashMap<String, WorkerInfo>,
}

impl Workers {
    pub fn new(config: &'static Config) -> Self {
        Self {
            config,
            workers: HashMap::new(),
        }
    }

    pub fn clean(&mut self) -> &mut Self {
        let now = OffsetDateTime::now_utc();
        self.workers
            .retain(|_, v| now <= v.last_seen + self.config.web_worker_timeout);
        self
    }

    pub fn verify(&self, name: &str, secret: &str) -> bool {
        let Some(worker) = self.workers.get(name) else { return true; };
        worker.secret == secret
    }

    pub fn update(&mut self, name: String, info: WorkerInfo) {
        self.workers.insert(name, info);
    }

    fn oldest_working_on(&self, hash: &str) -> Option<&str> {
        self.workers
            .iter()
            .filter_map(|(name, info)| match &info.status {
                WorkerStatus::Working(run) if run.hash == hash => Some((name, run.start)),
                _ => None,
            })
            .max_by_key(|(_, since)| *since)
            .map(|(name, _)| name as &str)
    }

    pub fn should_abort_work(&self, name: &str) -> bool {
        let Some(info) = self.workers.get(name) else { return false; };
        let WorkerStatus::Working ( run) = &info.status else { return false; };
        let Some(oldest) = self.oldest_working_on(&run.hash) else { return false; };
        name != oldest
    }

    pub fn find_free_work<'a>(&self, hashes: &'a [String]) -> Option<&'a str> {
        let covered = self
            .workers
            .values()
            .filter_map(|info| match &info.status {
                WorkerStatus::Working(run) => Some(&run.hash),
                _ => None,
            })
            .collect::<HashSet<_>>();

        hashes
            .iter()
            .find(|hash| !covered.contains(hash))
            .map(|hash| hash as &str)
    }

    pub fn get(&self, name: &str) -> Option<WorkerInfo> {
        self.workers.get(name).cloned()
    }

    pub fn get_all(&self) -> HashMap<String, WorkerInfo> {
        self.workers.clone()
    }
}
