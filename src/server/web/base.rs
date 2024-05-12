use maud::{html, Markup, DOCTYPE};

use crate::config::ServerConfig;

use super::{
    paths::{PathGraph, PathIndex, PathQueue},
    r#static::{BASE_CSS, LOGO_SVG},
    server_config_ext::{AbsPath, ServerConfigExt},
};

pub enum Tab {
    None,
    Index,
    Graph,
    Queue,
}

#[derive(Clone)]
pub struct Base {
    pub link_logo_svg: AbsPath,
    pub link_base_css: AbsPath,
    pub link_index: AbsPath,
    pub link_graph: AbsPath,
    pub link_queue: AbsPath,
    pub config: &'static ServerConfig,
    pub tab: &'static str,
}

impl Base {
    pub fn new(config: &'static ServerConfig, tab: Tab) -> Self {
        let tab = match tab {
            Tab::None => "",
            Tab::Index => "index",
            Tab::Graph => "graph",
            Tab::Queue => "queue",
        };
        Self {
            link_logo_svg: config.path(LOGO_SVG),
            link_base_css: config.path(BASE_CSS),
            link_index: config.path(PathIndex {}),
            link_graph: config.path(PathGraph {}),
            link_queue: config.path(PathQueue {}),
            config,
            tab,
        }
    }

    pub fn html(&self, title: &str, head: Markup, body: Markup) -> Markup {
        html!(
            (DOCTYPE)
            html lang="en" {
                head {
                    meta charset="utf-8";
                    meta name="viewport" content="width=device-width";
                    title { (title) " - " (self.config.repo_name) }
                    link rel="icon" href=(self.link_logo_svg);
                    link rel="stylesheet" href=(self.link_base_css);
                    (head)
                }
                body {
                    nav {
                        a .current[self.tab == "index"] href=(self.link_index) {
                            img src=(self.link_logo_svg) alt="";
                            (self.config.repo_name)
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
