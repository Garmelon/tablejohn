//! Coordinate performing runs across servers.

pub struct Coordinator {
    names: Vec<String>,
    current: usize,
}

impl Coordinator {
    pub fn new(mut names: Vec<String>) -> Self {
        assert!(!names.is_empty());
        names.sort_unstable();
        Self { names, current: 0 }
    }

    pub fn active(&self, name: &str) -> bool {
        self.names[self.current] == name
    }

    pub fn next(&mut self, name: &str) {
        // Check just to prevent weird shenanigans
        if self.active(name) {
            self.current += 1;
            self.current %= self.names.len();
        }
    }
}
