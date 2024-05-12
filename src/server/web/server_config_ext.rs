use std::fmt;

use crate::config::ServerConfig;

pub trait ServerConfigExt {
    fn path<T: fmt::Display>(&self, to: T) -> AbsPath;
}

impl ServerConfigExt for ServerConfig {
    fn path<T: fmt::Display>(&self, to: T) -> AbsPath {
        let to = to.to_string();
        assert!(to.starts_with('/'));
        AbsPath(format!("{}{to}", self.web_base))
    }
}

/// An absolute path to a resource on the web server.
#[derive(Clone)]
pub struct AbsPath(String);

impl fmt::Display for AbsPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl AsRef<str> for AbsPath {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
