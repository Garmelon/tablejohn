use axum_extra::routing::TypedPath;
use serde::Deserialize;

////////////////
// Html pages //
////////////////

#[derive(Deserialize, TypedPath)]
#[typed_path("/")]
pub struct PathIndex {}

#[derive(Deserialize, TypedPath)]
#[typed_path("/queue")]
pub struct PathQueue {}

#[derive(Deserialize, TypedPath)]
#[typed_path("/queue/inner")]
pub struct PathQueueInner {}

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

///////////////////
// Admin actions //
///////////////////

#[derive(Deserialize, TypedPath)]
#[typed_path("/admin/queue/add")]
pub struct PathAdminQueueAdd {}

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