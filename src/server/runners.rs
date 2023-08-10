use std::collections::HashMap;

use gix::hashtable::HashSet;
use time::OffsetDateTime;

use crate::{config::Config, shared::RunnerStatus};

#[derive(Clone)]
pub struct RunnerInfo {
    pub secret: String,
    pub last_seen: OffsetDateTime,
    pub status: RunnerStatus,
}

impl RunnerInfo {
    pub fn new(secret: String, last_seen: OffsetDateTime, status: RunnerStatus) -> Self {
        Self {
            secret,
            last_seen,
            status,
        }
    }
}

pub struct Runners {
    config: &'static Config,
    runners: HashMap<String, RunnerInfo>,
}

impl Runners {
    pub fn new(config: &'static Config) -> Self {
        Self {
            config,
            runners: HashMap::new(),
        }
    }

    pub fn clean(&mut self, now: OffsetDateTime) {
        self.runners
            .retain(|_, v| now <= v.last_seen + self.config.web_runner_timeout)
    }

    pub fn verify(&self, name: &str, secret: &str) -> bool {
        let Some(runner) = self.runners.get(name) else { return true; };
        runner.secret == secret
    }

    pub fn update(&mut self, name: String, info: RunnerInfo) {
        self.runners.insert(name, info);
    }

    fn oldest_working_on(&self, hash: &str) -> Option<&str> {
        self.runners
            .iter()
            .filter_map(|(name, info)| match &info.status {
                RunnerStatus::Working { hash: h, since, .. } if h == hash => Some((name, *since)),
                _ => None,
            })
            .max_by_key(|(_, since)| *since)
            .map(|(name, _)| name as &str)
    }

    pub fn should_abort_work(&self, name: &str) -> bool {
        let Some(info) = self.runners.get(name) else { return false; };
        let RunnerStatus::Working { hash, .. } = &info.status else { return false; };
        let Some(oldest) = self.oldest_working_on(hash) else { return false; };
        name != oldest
    }

    pub fn find_free_work<'a>(&self, hashes: &'a [String]) -> Option<&'a str> {
        let covered = self
            .runners
            .values()
            .filter_map(|info| match &info.status {
                RunnerStatus::Working { hash, .. } => Some(hash),
                _ => None,
            })
            .collect::<HashSet<_>>();

        hashes
            .iter()
            .find(|hash| !covered.contains(hash))
            .map(|hash| hash as &str)
    }

    pub fn get(&self, name: &str) -> Option<RunnerInfo> {
        self.runners.get(name).cloned()
    }

    pub fn get_all(&self) -> HashMap<String, RunnerInfo> {
        self.runners.clone()
    }
}
