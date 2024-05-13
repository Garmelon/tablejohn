use maud::{html, Markup, DOCTYPE};

use crate::{
    config::ServerConfig,
    server::web::{
        paths::{PathGraph, PathIndex, PathQueue},
        r#static::{LOGO_SVG, PAGE_CSS},
        server_config_ext::ServerConfigExt,
    },
};

#[derive(PartialEq, Eq)]
pub enum Tab {
    Index,
    Graph,
    Queue,
}

pub struct Page {
    config: &'static ServerConfig,
    title: String,
    tab: Option<Tab>,
    heads: Vec<Markup>,
    bodies: Vec<Markup>,
}

impl Page {
    pub fn new(config: &'static ServerConfig) -> Self {
        Self {
            config,
            title: "???".to_string(),
            tab: None,
            heads: vec![],
            bodies: vec![],
        }
    }

    pub fn title<T: ToString>(mut self, title: T) -> Self {
        self.title = title.to_string();
        self
    }

    pub fn tab(mut self, tab: Tab) -> Self {
        self.tab = Some(tab);
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
                    link rel="stylesheet" href=(self.config.path(PAGE_CSS));
                    @for head in self.heads { (head) }
                }
                body {
                    nav {
                        a .current[self.tab == Some(Tab::Index)] href=(self.config.path(PathIndex {})) {
                            img src=(self.config.path(LOGO_SVG)) alt="";
                            (self.config.repo_name)
                        }
                        a .current[self.tab == Some(Tab::Graph)] href=(self.config.path(PathGraph {})) { "graph" }
                        a .current[self.tab == Some(Tab::Queue)] href=(self.config.path(PathQueue {})) { "queue" }
                    }
                    @for body in self.bodies { (body) }
                }
            }
        )
    }
}
