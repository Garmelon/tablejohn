use askama::Template;
use maud::{html, Markup};
use time::OffsetDateTime;

use crate::server::util;

use super::{
    base::{Base, Link},
    paths::{PathCommitByHash, PathRunById, PathWorkerByName},
};

#[derive(Template)]
#[template(
    ext = "html",
    source = "\
{% import \"util.html\" as util %}
<a href=\"{{ link }}\" \
   class=\"{% call util::commit_class(reachable) %}\" \
   title=\"{% call util::commit_title(reachable) %}\">
   {{ short|truncate(80) }}
</a>
"
)]
pub struct LinkCommit {
    link: Link,
    short: String,
    reachable: i64,
}

impl LinkCommit {
    pub fn new(base: &Base, hash: String, message: &str, reachable: i64) -> Self {
        Self {
            short: util::format_commit_short(&hash, message),
            link: base.link(PathCommitByHash { hash }),
            reachable,
        }
    }

    pub fn class_and_title(&self) -> (&'static str, &'static str) {
        if self.reachable == 0 {
            (
                "commit-orphaned",
                "This commit is orphaned. It can't be reached from any ref.",
            )
        } else if self.reachable == -1 {
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

    pub fn html(&self) -> Markup {
        let (class, title) = self.class_and_title();

        let truncate = self.short.chars().take(81).count() > 80;
        let short = if truncate {
            self.short
                .chars()
                .take(80 - 3)
                .chain("...".chars())
                .collect::<String>()
        } else {
            self.short.to_string()
        };

        html! {
            a href=(self.link) .(class) title=(title) { (short) }
        }
    }
}

#[derive(Template)]
#[template(
    ext = "html",
    source = "<a href=\"{{ link }}\">Run of {{ short|truncate(80) }}</a>"
)]
pub struct LinkRunShort {
    link: Link,
    short: String,
}

impl LinkRunShort {
    pub fn new(base: &Base, id: String, hash: &str, message: &str) -> Self {
        Self {
            link: base.link(PathRunById { id }),
            short: util::format_commit_short(hash, message),
        }
    }
}

#[derive(Template)]
#[template(
    ext = "html",
    source = "<a href=\"{{ link }}\">Run from {{ date }}</a>"
)]
pub struct LinkRunDate {
    link: Link,
    date: String, // TODO base.date(...)?
}

impl LinkRunDate {
    pub fn new(base: &Base, id: String, start: OffsetDateTime) -> Self {
        Self {
            link: base.link(PathRunById { id }),
            date: util::format_time(start),
        }
    }

    pub fn html(&self) -> Markup {
        html! {
            a href=(self.link) { "Run from " (self.date) }
        }
    }
}

#[derive(Template)]
#[template(ext = "html", source = "<a href=\"{{ link }}\">{{ name }}</a>")]
pub struct LinkWorker {
    link: Link,
    name: String,
}

impl LinkWorker {
    pub fn new(base: &Base, name: String) -> Self {
        Self {
            link: base.link(PathWorkerByName { name: name.clone() }),
            name,
        }
    }
}
