use askama::Template;

use crate::server::util;

use super::Base;

#[derive(Template)]
#[template(
    ext = "html",
    source = "\
{% import \"util.html\" as util %}
<a href=\"{{ root }}commit/{{ hash }}\"
   class=\"{% call util::commit_class(reachable) %}\"
   title=\"{% call util::commit_title(reachable) %}\">
   {{ short }}
</a>
"
)]
pub struct CommitLink {
    root: String,
    hash: String,
    short: String,
    reachable: i64,
}

impl CommitLink {
    pub fn new(base: &Base, hash: String, message: &str, reachable: i64) -> Self {
        Self {
            root: base.root.clone(),
            short: util::format_commit_short(&hash, message),
            hash,
            reachable,
        }
    }
}

#[derive(Template)]
#[template(
    ext = "html",
    source = "\
<a href=\"{{ root }}run/{{ id }}\">
   Run of {{ short }}
</a>
"
)]
pub struct RunLink {
    root: String,
    id: String,
    short: String,
}

impl RunLink {
    pub fn new(base: &Base, id: String, hash: &str, message: &str) -> Self {
        Self {
            root: base.root.clone(),
            id,
            short: util::format_commit_short(hash, message),
        }
    }
}
#[derive(Template)]
#[template(
    ext = "html",
    source = "\
<a href=\"{{ root }}worker/{{ name }}\">
   {{ name }}
</a>
"
)]
pub struct WorkerLink {
    root: String,
    name: String,
}

impl WorkerLink {
    pub fn new(base: &Base, name: String) -> Self {
        Self {
            root: base.root.clone(),
            name,
        }
    }
}
