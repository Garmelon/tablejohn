//! Configuration from a file.

use std::{fs, io::ErrorKind, path::Path, time::Duration};

use serde::Deserialize;
use tracing::{debug, info};

mod default {
    use std::time::Duration;

    pub fn repo_update_delay() -> Duration {
        Duration::from_secs(60)
    }
}

#[derive(Debug, Deserialize)]
pub struct Repo {
    #[serde(default = "default::repo_update_delay", with = "humantime_serde")]
    pub update_delay: Duration,
}

impl Default for Repo {
    fn default() -> Self {
        Self {
            update_delay: default::repo_update_delay(),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct Config {
    pub repo: Repo,
}

impl Config {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        info!(path = %path.display(), "Loading config");
        let config = match fs::read_to_string(path) {
            Ok(str) => toml::from_str(&str)?,
            Err(e) if e.kind() == ErrorKind::NotFound => {
                info!("No config file found, using default config");
                Self::default()
            }
            Err(e) => Err(e)?,
        };

        debug!("Loaded config:\n{config:#?}");
        Ok(config)
    }
}
