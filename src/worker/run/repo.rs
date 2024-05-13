use crate::{somehow, worker::server::Server};

use super::{Finished, RunInProgress};

impl RunInProgress {
    pub(super) async fn execute_repo(
        &self,
        server: &Server,
        hash: &str,
    ) -> somehow::Result<Option<Finished>> {
        // TODO Design bench repo specification (benchmark, compare)
        // TODO Decide on better name? "benchmark repo", "bench repo", "eval repo"?
        // TODO Implement specification
        let repo_dir = server.download_repo(&self.run.hash).await?;
        let bench_repo_dir = server.download_bench_repo(hash).await?;

        todo!()
    }
}
