use std::fmt;

use maud::{html, Markup, DOCTYPE};

use crate::config::ServerConfig;

use super::{
    paths::{PathGraph, PathIndex, PathQueue},
    r#static::{BASE_CSS, LOGO_SVG},
};

pub enum Tab {
    None,
    Index,
    Graph,
    Queue,
}

#[derive(Clone)]
pub struct Base {
    pub link_logo_svg: Link,
    pub link_base_css: Link,
    pub link_index: Link,
    pub link_graph: Link,
    pub link_queue: Link,
    pub web_base: String,
    pub repo_name: String,
    pub tab: &'static str,
}

impl Base {
    pub fn new(config: &ServerConfig, tab: Tab) -> Self {
        let tab = match tab {
            Tab::None => "",
            Tab::Index => "index",
            Tab::Graph => "graph",
            Tab::Queue => "queue",
        };
        Self {
            link_logo_svg: Self::link_with_config(config, LOGO_SVG),
            link_base_css: Self::link_with_config(config, BASE_CSS),
            link_index: Self::link_with_config(config, PathIndex {}),
            link_graph: Self::link_with_config(config, PathGraph {}),
            link_queue: Self::link_with_config(config, PathQueue {}),
            web_base: config.web_base.clone(),
            repo_name: config.repo_name.clone(),
            tab,
        }
    }

    fn link_with_base<P: fmt::Display>(base: &str, to: P) -> Link {
        let to = format!("{to}");
        assert!(!base.ends_with('/'));
        assert!(to.starts_with('/'));
        Link(format!("{base}{to}"))
    }

    pub fn link_with_config<P: fmt::Display>(config: &ServerConfig, to: P) -> Link {
        Self::link_with_base(&config.web_base, to)
    }

    pub fn link<P: fmt::Display>(&self, to: P) -> Link {
        Self::link_with_base(&self.web_base, to)
    }

    pub fn html(&self, title: &str, head: Markup, body: Markup) -> Markup {
        html!(
            (DOCTYPE)
            html lang="en" {
                head {
                    meta charset="utf-8";
                    meta name="viewport" content="width=device-width";
                    title { (title) " - " (self.repo_name) }
                    link rel="icon" href=(self.link_logo_svg);
                    link rel="stylesheet" href=(self.link_base_css);
                    (head)
                }
                body {
                    nav {
                        a .current[self.tab == "index"] href=(self.link_index) {
                            img src=(self.link_logo_svg) alt="";
                            (self.repo_name)
                        }
                        a .current[self.tab == "graph"] href=(self.link_graph) { "graph" }
                        a .current[self.tab == "queue"] href=(self.link_queue) { "queue" }
                    }
                    (body)
                }
            }
        )
    }
}

#[derive(Clone)]
pub struct Link(String);

impl fmt::Display for Link {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
