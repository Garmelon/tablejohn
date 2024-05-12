use maud::{html, Markup};
use time::OffsetDateTime;

use crate::{config::ServerConfig, server::util};

use super::{
    paths::{PathCommitByHash, PathRunById, PathWorkerByName},
    server_config_ext::ServerConfigExt,
};

pub fn join(sections: &[Markup], with: Markup) -> Markup {
    html! {
        @for (i, section) in sections.iter().enumerate() {
            @if i > 0 { (with) }
            (section)
        }
    }
}

pub fn commit_class_and_title(reachable: i64) -> (&'static str, &'static str) {
    if reachable == 0 {
        (
            "commit-orphaned",
            "This commit is orphaned. It can't be reached from any ref.",
        )
    } else if reachable == -1 {
        (
            "commit-reachable",
            "This commit can only be reached from untracked refs.",
        )
    } else {
        (
            "commit-tracked",
            "This commit can be reached from a tracked ref.",
        )
    }
}

pub fn link_commit(config: &ServerConfig, hash: String, message: &str, reachable: i64) -> Markup {
    let short = util::truncate(&util::format_commit_short(&hash, message), 80);
    let path = config.path(PathCommitByHash { hash });
    let (class, title) = commit_class_and_title(reachable);

    html! {
        a href=(path) .(class) title=(title) { (short) }
    }
}

/// Link to a run by its commit's short message.
pub fn link_run_short(config: &ServerConfig, id: String, hash: &str, message: &str) -> Markup {
    let short = util::truncate(&util::format_commit_short(hash, message), 80);
    let path = config.path(PathRunById { id });

    html! {
        a href=(path) { "Run of " (short) }
    }
}

/// Link to a run by its start time.
pub fn link_run_date(config: &ServerConfig, id: String, start: OffsetDateTime) -> Markup {
    let start = util::format_time(start);
    let path = config.path(PathRunById { id });

    html! {
        a href=(path) { "Run from " (start) }
    }
}

pub fn link_worker(config: &ServerConfig, name: String) -> Markup {
    let path = config.path(PathWorkerByName { name: name.clone() });

    html! {
        a href=(path) { (name) }
    }
}
