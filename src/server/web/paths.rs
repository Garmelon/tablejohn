use axum_extra::routing::TypedPath;
use serde::Deserialize;

////////////////
// Html pages //
////////////////

#[derive(Deserialize, TypedPath)]
#[typed_path("/")]
pub struct PathIndex {}

#[derive(Deserialize, TypedPath)]
#[typed_path("/graph/")]
pub struct PathGraph {}

#[derive(Deserialize, TypedPath)]
#[typed_path("/graph/metrics")]
pub struct PathGraphMetrics {}

#[derive(Deserialize, TypedPath)]
#[typed_path("/graph/commits")]
pub struct PathGraphCommits {}

#[derive(Deserialize, TypedPath)]
#[typed_path("/graph/measurements")]
pub struct PathGraphMeasurements {}

#[derive(Deserialize, TypedPath)]
#[typed_path("/queue/")]
pub struct PathQueue {}

#[derive(Deserialize, TypedPath)]
#[typed_path("/queue/inner")]
pub struct PathQueueInner {}

#[derive(Deserialize, TypedPath)]
#[typed_path("/queue/delete/:hash")]
pub struct PathQueueDelete {
    pub hash: String,
}

#[derive(Deserialize, TypedPath)]
#[typed_path("/commit/:hash")]
pub struct PathCommitByHash {
    pub hash: String,
}

#[derive(Deserialize, TypedPath)]
#[typed_path("/run/:id")]
pub struct PathRunById {
    pub id: String,
}

#[derive(Deserialize, TypedPath)]
#[typed_path("/worker/:name")]
pub struct PathWorkerByName {
    pub name: String,
}

#[derive(Deserialize, TypedPath)]
#[typed_path("/test")]
pub struct PathTest {}

///////////////////
// Admin actions //
///////////////////

#[derive(Deserialize, TypedPath)]
#[typed_path("/admin/refs/track")]
pub struct PathAdminRefsTrack {}

#[derive(Deserialize, TypedPath)]
#[typed_path("/admin/refs/untrack")]
pub struct PathAdminRefsUntrack {}

#[derive(Deserialize, TypedPath)]
#[typed_path("/admin/refs/update")]
pub struct PathAdminRefsUpdate {}

#[derive(Deserialize, TypedPath)]
#[typed_path("/admin/queue/add")]
pub struct PathAdminQueueAdd {}

#[derive(Deserialize, TypedPath)]
#[typed_path("/admin/queue/add_batch")]
pub struct PathAdminQueueAddBatch {}

#[derive(Deserialize, TypedPath)]
#[typed_path("/admin/queue/delete")]
pub struct PathAdminQueueDelete {}

#[derive(Deserialize, TypedPath)]
#[typed_path("/admin/queue/increase")]
pub struct PathAdminQueueIncrease {}

#[derive(Deserialize, TypedPath)]
#[typed_path("/admin/queue/decrease")]
pub struct PathAdminQueueDecrease {}

/////////
// Api //
/////////

#[derive(Deserialize, TypedPath)]
#[typed_path("/api/worker/status")]
pub struct PathApiWorkerStatus {}

#[derive(Deserialize, TypedPath)]
#[typed_path("/api/worker/repo/:hash/tree.tar.gz")]
pub struct PathApiWorkerRepoByHashTreeTarGz {
    pub hash: String,
}

#[derive(Deserialize, TypedPath)]
#[typed_path("/api/worker/bench_repo/:hash/tree.tar.gz")]
pub struct PathApiWorkerBenchRepoByHashTreeTarGz {
    pub hash: String,
}
