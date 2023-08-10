//! Coordinate performing runs across servers.

use tokio::sync::mpsc;

struct Server {
    name: String,
    poke: mpsc::UnboundedSender<()>,
}

pub struct Coordinator {
    servers: Vec<Server>,
    current: usize,
}

impl Coordinator {
    pub fn new() -> Self {
        Self {
            servers: vec![],
            current: 0,
        }
    }

    pub fn register(&mut self, name: String, poke: mpsc::UnboundedSender<()>) {
        self.servers.push(Server { name, poke });
    }

    pub fn active(&self, name: &str) -> bool {
        if let Some(current) = self.servers.get(self.current) {
            name == current.name
        } else {
            false
        }
    }

    pub fn next(&mut self, name: &str) {
        // Check just to prevent weird shenanigans
        if !self.active(name) {
            return;
        }

        // At least one server (the current one) must be registered according to
        // the previous check
        assert!(!self.servers.is_empty());

        self.current += 1;
        self.current %= self.servers.len();

        // When the runner seeks work and a queue is idle, the next server
        // should be queried immediately. Otherwise, we'd introduce lots of
        // delay in the multi-server case were most queues are empty.
        //
        // However, if all server's queues were empty, this would generate a
        // slippery cycle of requests that the runner sends as quickly as
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
        if self.current > 0 {
            let _ = self.servers[self.current].poke.send(());
        }
    }
}
