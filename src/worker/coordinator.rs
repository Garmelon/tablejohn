//! Coordinate performing runs across servers.

use std::time::Duration;

use time::OffsetDateTime;
use tokio::sync::mpsc;

struct Server {
    name: String,
    poke: mpsc::UnboundedSender<()>,
}

pub struct Coordinator {
    servers: Vec<Server>,
    active: usize,
    active_since: OffsetDateTime,
    busy: bool,
}

impl Coordinator {
    pub fn new() -> Self {
        Self {
            servers: vec![],
            active: 0,
            active_since: OffsetDateTime::now_utc(),
            busy: false,
        }
    }

    pub fn register(&mut self, name: String, poke: mpsc::UnboundedSender<()>) {
        // TODO Assert that no duplicate names exist?
        self.servers.push(Server { name, poke });
    }

    pub fn active(&self, name: &str) -> ActiveInfo {
        let active_server = self.servers.get(self.active);
        let active = active_server.filter(|s| s.name == name).is_some();
        ActiveInfo {
            active,
            active_since: self.active_since,
            busy: self.busy,
        }
    }

    pub fn look_busy(&mut self, name: &str) {
        // Check just to prevent weird shenanigans
        if !self.active(name).active {
            return;
        }

        self.busy = true;
    }

    pub fn move_to_next_server(&mut self, name: &str) {
        // Check just to prevent weird shenanigans
        if !self.active(name).active {
            return;
        }

        // At least one server (the current one) must be registered according to
        // the previous check
        assert!(!self.servers.is_empty());

        self.active += 1;
        self.active %= self.servers.len();
        self.active_since = OffsetDateTime::now_utc();
        self.busy = false;

        // When the worker seeks work and a queue is idle, the next server
        // should be queried immediately. Otherwise, we'd introduce lots of
        // delay in the multi-server case were most queues are empty.
        //
        // However, if all server's queues were empty, this would generate a
        // slippery cycle of requests that the worker sends as quickly as
        // possible, only limited by the roundtrip time. Because we don't want
        // this, we let the first task wait its full timeout. Effectively, this
        // results in iterations starting at least the ping delay apart, which
        // is pretty much what we want.
        //
        // The way this is implemented currently is sub-optimal however: If the
        // chain takes even a fraction longer than the previous iteration, tasks
        // will send two requests back-to-back: The first because their ping
        // timeout ran out, and the second because they were poked. So far, I
        // haven't been able to think of an elegant solution for this.
        if self.active > 0 {
            let _ = self.servers[self.active].poke.send(());
        }
    }
}

#[derive(Clone, Copy)]
pub struct ActiveInfo {
    pub active: bool,
    pub active_since: OffsetDateTime,
    pub busy: bool,
}

impl ActiveInfo {
    pub fn in_batch(&self, batch_duration: Duration) -> bool {
        let batch_end = self.active_since + batch_duration;
        let now = OffsetDateTime::now_utc();
        now <= batch_end
    }
}
