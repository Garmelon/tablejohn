use std::fmt;

use crate::config::Config;

pub enum Tab {
    None,
    Index,
    Queue,
}

#[derive(Clone)]
pub struct Base {
    web_base: String,
    repo_name: String,
    tab: &'static str,
}

impl Base {
    pub fn new(config: &Config, tab: Tab) -> Self {
        let tab = match tab {
            Tab::None => "",
            Tab::Index => "index",
            Tab::Queue => "queue",
        };
        Self {
            web_base: config.web_base.clone(),
            repo_name: config.repo_name.clone(),
            tab,
        }
    }

    pub fn link<P: fmt::Display>(&self, to: P) -> Link {
        let to = format!("{to}");
        assert!(!self.web_base.ends_with('/'));
        assert!(to.starts_with('/'));
        Link(format!("{}{to}", self.web_base))
    }
}

pub struct Link(String);

impl fmt::Display for Link {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
