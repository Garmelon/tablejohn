use tracing::error;

pub struct Runner {}

impl Runner {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn run(&self) {
        error!("Runner not yet implemented");
    }
}
