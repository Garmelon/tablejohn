use axum::{extract::State, response::IntoResponse};
use futures::TryStreamExt;
use maud::html;
use sqlx::SqlitePool;

use crate::{
    config::ServerConfig,
    server::web::{
        base::{Base, Tab},
        link::LinkCommit,
        paths::{PathAdminRefsTrack, PathAdminRefsUntrack, PathAdminRefsUpdate, PathIndex},
    },
    somehow,
};

struct Ref {
    name: String,
    commit: LinkCommit,
    tracked: bool,
}

pub async fn get_index(
    _path: PathIndex,
    State(config): State<&'static ServerConfig>,
    State(db): State<SqlitePool>,
) -> somehow::Result<impl IntoResponse> {
    let base = Base::new(config, Tab::Index);

    let refs = sqlx::query!(
        "\
        SELECT name, hash, message, reachable, tracked \
        FROM refs \
        JOIN commits USING (hash) \
        ORDER BY name ASC \
        "
    )
    .fetch(&db)
    .map_ok(|r| Ref {
        name: r.name.clone(),
        commit: LinkCommit::new(&base, r.hash, &r.message, r.reachable),
        tracked: r.tracked != 0,
    })
    .try_collect::<Vec<_>>()
    .await?;

    let mut tracked_refs = vec![];
    let mut untracked_refs = vec![];
    for reference in refs {
        if reference.tracked {
            tracked_refs.push(reference);
        } else {
            untracked_refs.push(reference);
        }
    }

    Ok(base.html(
        "overview",
        html! {},
        html! {
            h2 { "Refs" }
            details .refs-list open {
                summary { "Tracked (" (tracked_refs.len()) ")" }
                form method="post" action=(base.link(PathAdminRefsUntrack {})) {
                    dl {
                        @for r#ref in tracked_refs {
                            dt {
                                (r#ref.name) " ["
                                button .linkish name="ref" value=(r#ref.name) { "untrack" }
                                "]"
                            }
                            dd { (r#ref.commit.html()) }
                        }
                    }
                }
            }
            details .refs-list {
                summary { "Untracked (" (untracked_refs.len()) ")" }
                form method="post" action=(base.link(PathAdminRefsTrack {})) {
                    dl {
                        @for r#ref in untracked_refs {
                            dt {
                                (r#ref.name) " ["
                                button .linkish name="ref" value=(r#ref.name) { "track" }
                                "]"
                            }
                            dd { (r#ref.commit.html()) }
                        }
                    }
                }
            }
            form method="post" action=(base.link(PathAdminRefsUpdate {})) {
                button { "Update" }
            }
        },
    ))
}
