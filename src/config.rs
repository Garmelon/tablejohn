//! Configuration from a file.

use std::{
    collections::HashMap,
    fs,
    io::ErrorKind,
    net::SocketAddr,
    path::{Path, PathBuf},
    time::Duration,
};

use directories::ProjectDirs;
use serde::Deserialize;
use tracing::{debug, info};

use crate::{
    args::{Args, Command, ServerCommand},
    somehow,
};

mod default {
    use std::{net::SocketAddr, time::Duration};

    pub fn web_base() -> String {
        "".to_string()
    }

    pub fn web_address() -> SocketAddr {
        // Port chosen by fair dice roll
        "[::1]:8221".parse().unwrap()
    }

    pub fn repo_update_delay() -> Duration {
        Duration::from_secs(60)
    }

    pub fn runner_ping_delay() -> Duration {
        Duration::from_secs(10)
    }
}

#[derive(Debug, Deserialize)]
struct Web {
    #[serde(default = "default::web_base")]
    base: String,
    #[serde(default = "default::web_address")]
    address: SocketAddr,
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
struct Repo {
    name: Option<String>,
    #[serde(default = "default::repo_update_delay", with = "humantime_serde")]
    update_delay: Duration,
}

impl Default for Repo {
    fn default() -> Self {
        Self {
            name: None,
            update_delay: default::repo_update_delay(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct RunnerServer {
    url: String,
    token: String,
}

#[derive(Debug, Deserialize)]
struct Runner {
    name: Option<String>,
    #[serde(default = "default::runner_ping_delay", with = "humantime_serde")]
    ping_delay: Duration,
    servers: HashMap<String, RunnerServer>,
}

impl Default for Runner {
    fn default() -> Self {
        Self {
            name: None,
            ping_delay: default::runner_ping_delay(),
            servers: HashMap::new(),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
struct ConfigFile {
    #[serde(default)]
    web: Web,
    #[serde(default)]
    repo: Repo,
    #[serde(default)]
    runner: Runner,
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

    fn repo_name(&self, args: &Args) -> somehow::Result<String> {
        if let Some(name) = &self.repo.name {
            return Ok(name.clone());
        }

        if let Command::Server(ServerCommand {
            repo: Some(path), ..
        }) = &args.command
        {
            if let Some(name) = path.canonicalize()?.file_name() {
                let name = name.to_string_lossy();
                let name = name.strip_suffix(".git").unwrap_or(&name).to_string();
                return Ok(name);
            }
        }

        Ok("unnamed repo".to_string())
    }

    fn runner_name(&self) -> String {
        if let Some(name) = &self.runner.name {
            return name.clone();
        }

        gethostname::gethostname().to_string_lossy().to_string()
    }

    fn runner_servers(&self) -> HashMap<String, RunnerServerConfig> {
        self.runner
            .servers
            .iter()
            .map(|(name, server)| {
                let url = server
                    .url
                    .strip_suffix('/')
                    .unwrap_or(&server.url)
                    .to_string();
                let token = server.token.to_string();
                (name.to_string(), RunnerServerConfig { url, token })
            })
            .collect()
    }
}

pub struct RunnerServerConfig {
    pub url: String,
    pub token: String,
}

pub struct Config {
    pub web_base: String,
    pub web_address: SocketAddr,
    pub repo_name: String,
    pub repo_update_delay: Duration,
    pub runner_name: String,
    pub runner_ping_delay: Duration,
    pub runner_servers: HashMap<String, RunnerServerConfig>,
}

impl Config {
    fn path(args: &Args) -> PathBuf {
        if let Some(path) = &args.config {
            return path.clone();
        }

        ProjectDirs::from("de", "plugh", "tablejohn")
            .expect("could not determine home directory")
            .config_dir()
            .join("config.toml")
    }

    pub fn load(args: &Args) -> somehow::Result<Self> {
        let path = Self::path(args);
        info!(path = %path.display(), "Loading config");
        let config_file = ConfigFile::load(&path)?;
        debug!("Loaded config file:\n{config_file:#?}");

        let web_base = config_file.web_base();
        let repo_name = config_file.repo_name(args)?;
        let runner_name = config_file.runner_name();
        let runner_servers = config_file.runner_servers();

        Ok(Self {
            web_base,
            web_address: config_file.web.address,
            repo_name,
            repo_update_delay: config_file.repo.update_delay,
            runner_name,
            runner_ping_delay: config_file.runner.ping_delay,
            runner_servers,
        })
    }
}
