use maud::{html, Markup, DOCTYPE};

use crate::{
    config::ServerConfig,
    server::web::{
        paths::{PathGraph, PathIndex, PathQueue},
        r#static::{BASE_CSS, LOGO_SVG},
        server_config_ext::ServerConfigExt,
    },
};

#[derive(PartialEq, Eq)]
pub enum Tab {
    None,
    Index,
    Graph,
    Queue,
}

pub struct Page {
    config: &'static ServerConfig,
    title: String,
    nav: Option<Markup>,
    heads: Vec<Markup>,
    bodies: Vec<Markup>,
}

impl Page {
    pub fn new(config: &'static ServerConfig) -> Self {
        Self {
            config,
            title: "???".to_string(),
            nav: None,
            heads: vec![],
            bodies: vec![],
        }
    }

    pub fn title<T: ToString>(mut self, title: T) -> Self {
        self.title = title.to_string();
        self
    }

    pub fn nav(mut self, tab: Tab) -> Self {
        self.nav = Some(html! {
            nav {
                a .current[tab == Tab::Index] href=(self.config.path(PathIndex {})) {
                    img src=(self.config.path(LOGO_SVG)) alt="";
                    (self.config.repo_name)
                }
                a .current[tab == Tab::Graph] href=(self.config.path(PathGraph {})) { "graph" }
                a .current[tab == Tab::Queue] href=(self.config.path(PathQueue {})) { "queue" }
            }
        });

        self
    }

    pub fn head(mut self, head: Markup) -> Self {
        self.heads.push(head);
        self
    }

    pub fn body(mut self, body: Markup) -> Self {
        self.bodies.push(body);
        self
    }

    pub fn build(self) -> Markup {
        html!(
            (DOCTYPE)
            html lang="en" {
                head {
                    meta charset="utf-8";
                    meta name="viewport" content="width=device-width";
                    title { (self.title) " - " (self.config.repo_name) }
                    link rel="icon" href=(self.config.path(LOGO_SVG));
                    link rel="stylesheet" href=(self.config.path(BASE_CSS));
                    @if let Some(nav) = self.nav { (nav) }
                    @for head in self.heads { (head) }
                }
                body {
                    @for body in self.bodies { (body) }
                }
            }
        )
    }
}
