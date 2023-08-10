use askama::Template;

use crate::server::util;

use super::Base;

#[derive(Template)]
#[template(
    ext = "html",
    source = "
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
