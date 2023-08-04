//! Configuration from a file.

use std::{fs, io::ErrorKind, path::Path};

use serde::Deserialize;
use tracing::info;

#[derive(Debug, Default, Deserialize)]
pub struct Config {}

impl Config {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        info!("Loading config from {}", path.display());
        Ok(match fs::read_to_string(path) {
            Ok(str) => toml::from_str(&str)?,
            Err(e) if e.kind() == ErrorKind::NotFound => {
                info!("No config file found, using default config");
                Self::default()
            }
            Err(e) => Err(e)?,
        })
    }
}
