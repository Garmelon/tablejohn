//! Configuration from a file.

use std::{fs, io::ErrorKind, net::SocketAddr, path::Path, time::Duration};

use serde::Deserialize;
use tracing::{debug, info};

use crate::somehow;

mod default {
    use std::{net::SocketAddr, time::Duration};

    pub fn web_base() -> String {
        "".to_string()
    }

    pub fn web_address() -> SocketAddr {
        // Port chosen by fair dice roll
        "[::]:8221".parse().unwrap()
    }

    pub fn repo_name() -> String {
        "local repo".to_string()
    }

    pub fn repo_update_delay() -> Duration {
        Duration::from_secs(60)
    }
}

#[derive(Debug, Deserialize)]
pub struct Web {
    #[serde(default = "default::web_base")]
    pub base: String,
    #[serde(default = "default::web_address")]
    pub address: SocketAddr,
}

impl Default for Web {
    fn default() -> Self {
        Self {
            base: default::web_base(),
            address: default::web_address(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Repo {
    #[serde(default = "default::repo_name")]
    pub name: String,
    #[serde(default = "default::repo_update_delay", with = "humantime_serde")]
    pub update_delay: Duration,
}

impl Default for Repo {
    fn default() -> Self {
        Self {
            name: default::repo_name(),
            update_delay: default::repo_update_delay(),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
struct ConfigFile {
    repo: Repo,
    web: Web,
}

impl ConfigFile {
    fn load(path: &Path) -> somehow::Result<Self> {
        let config = match fs::read_to_string(path) {
            Ok(str) => toml::from_str(&str)?,
            Err(e) if e.kind() == ErrorKind::NotFound => {
                info!("No config file found, using default config");
                Self::default()
            }
            Err(e) => Err(e)?,
        };

        Ok(config)
    }

    fn web_base(&self) -> String {
        self.web
            .base
            .strip_prefix('/')
            .unwrap_or(&self.web.base)
            .strip_suffix('/')
            .unwrap_or(&self.web.base)
            .to_string()
    }
}

pub struct Config {
    pub web_base: String,
    pub web_address: SocketAddr,
    pub repo_name: String,
    pub repo_update_delay: Duration,
}

impl Config {
    pub fn load(path: &Path) -> somehow::Result<Self> {
        info!(path = %path.display(), "Loading config");
        let config_file = ConfigFile::load(path)?;
        debug!("Loaded config file:\n{config_file:#?}");

        let web_base = config_file.web_base();

        Ok(Self {
            web_base,
            web_address: config_file.web.address,
            repo_name: config_file.repo.name,
            repo_update_delay: config_file.repo.update_delay,
        })
    }
}
