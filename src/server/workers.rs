use std::collections::{HashMap, HashSet};

use time::OffsetDateTime;

use crate::{
    config::Config,
    id,
    shared::{BenchMethod, UnfinishedRun, Work, WorkerStatus},
};

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

    /// Find and reserve work for a worker.
    pub fn find_work(&mut self, name: &str, queue: &[String], bench: BenchMethod) -> Option<Work> {
        let covered = self
            .workers
            .values()
            .filter_map(|info| match &info.status {
                WorkerStatus::Idle | WorkerStatus::Busy => None,
                WorkerStatus::Working(run) => Some(&run.hash),
            })
            .collect::<HashSet<_>>();

        // Find work not already covered by another worker
        let hash = queue.iter().find(|hash| !covered.contains(hash))?.clone();
        let id = id::random_run_id();
        let work = Work { id, hash, bench };

        // Reserve work so other workers don't choose it
        if let Some(info) = self.workers.get_mut(name) {
            info.status = WorkerStatus::Working(UnfinishedRun {
                id: work.id.clone(),
                hash: work.hash.clone(),
                start: OffsetDateTime::now_utc(),
                last_output: vec![],
            });
        }

        Some(work)
    }

    pub fn should_abort_work(&self, name: &str, queue: &[String]) -> bool {
        // A runner should abort work if...
        let Some(info) = self.workers.get(name) else { return false; };
        let WorkerStatus::Working (run) = &info.status else { return false; };

        // The commit isn't in the queue
        if !queue.contains(&run.hash) {
            return true;
        }

        // Another runner has been working on the same commit for longer
        let oldest_working_on_commit = self
            .workers
            .iter()
            .filter_map(|(name, info)| match &info.status {
                WorkerStatus::Working(r) if r.hash == run.hash => Some((name, r.start)),
                _ => None,
            })
            .max_by_key(|(_, start)| *start)
            .map(|(name, _)| name as &str);
        if oldest_working_on_commit != Some(name) {
            return true;
        }

        false
    }

    pub fn get(&self, name: &str) -> Option<WorkerInfo> {
        self.workers.get(name).cloned()
    }

    pub fn get_all(&self) -> HashMap<String, WorkerInfo> {
        self.workers.clone()
    }
}
