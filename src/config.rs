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
    id, somehow,
};

mod default {
    use std::{net::SocketAddr, time::Duration};

    pub fn web_base() -> String {
        "/".to_string()
    }

    pub fn web_address() -> SocketAddr {
        // Port chosen by fair dice roll
        "[::1]:8221".parse().unwrap()
    }

    pub fn web_worker_timeout() -> Duration {
        Duration::from_secs(60)
    }

    pub fn repo_update_delay() -> Duration {
        Duration::from_secs(60)
    }

    pub fn worker_ping_delay() -> Duration {
        Duration::from_secs(10)
    }
}

#[derive(Debug, Deserialize)]
struct Web {
    #[serde(default = "default::web_base")]
    base: String,

    #[serde(default = "default::web_address")]
    address: SocketAddr,

    worker_token: Option<String>,

    #[serde(default = "default::web_worker_timeout")]
    worker_timeout: Duration,
}

impl Default for Web {
    fn default() -> Self {
        Self {
            base: default::web_base(),
            address: default::web_address(),
            worker_token: None,
            worker_timeout: default::web_worker_timeout(),
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
struct WorkerServer {
    url: String,
    token: String,
}

#[derive(Debug, Deserialize)]
struct Worker {
    name: Option<String>,

    #[serde(default = "default::worker_ping_delay", with = "humantime_serde")]
    ping_delay: Duration,

    servers: HashMap<String, WorkerServer>,
}

impl Default for Worker {
    fn default() -> Self {
        Self {
            name: None,
            ping_delay: default::worker_ping_delay(),
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
    worker: Worker,
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
        let mut base = self.web.base.clone();
        if !base.starts_with('/') {
            base.insert(0, '/');
        }
        if !base.ends_with('/') {
            base.push('/');
        }
        base
    }

    fn web_worker_token(&self) -> String {
        self.web
            .worker_token
            .clone()
            .unwrap_or_else(id::random_worker_token)
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

    fn worker_name(&self) -> String {
        if let Some(name) = &self.worker.name {
            return name.clone();
        }

        gethostname::gethostname().to_string_lossy().to_string()
    }

    fn worker_servers(&self) -> HashMap<String, WorkerServerConfig> {
        self.worker
            .servers
            .iter()
            .map(|(name, server)| {
                let mut url = server.url.clone();
                if !url.ends_with('/') {
                    url.push('/');
                }
                let token = server.token.to_string();
                (name.to_string(), WorkerServerConfig { url, token })
            })
            .collect()
    }
}

#[derive(Clone)]
pub struct WorkerServerConfig {
    /// Always ends with a `/`.
    pub url: String,
    pub token: String,
}

#[derive(Clone)]
pub struct Config {
    /// Always starts and ends with a `/`.
    pub web_base: String,
    pub web_address: SocketAddr,
    pub web_worker_token: String,
    pub web_worker_timeout: Duration,
    pub repo_name: String,
    pub repo_update_delay: Duration,
    pub worker_name: String,
    pub worker_ping_delay: Duration,
    pub worker_servers: HashMap<String, WorkerServerConfig>,
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
        let web_worker_token = config_file.web_worker_token();
        let repo_name = config_file.repo_name(args)?;
        let worker_name = config_file.worker_name();
        let worker_servers = config_file.worker_servers();

        Ok(Self {
            web_base,
            web_address: config_file.web.address,
            web_worker_token,
            web_worker_timeout: config_file.web.worker_timeout,
            repo_name,
            repo_update_delay: config_file.repo.update_delay,
            worker_name,
            worker_ping_delay: config_file.worker.ping_delay,
            worker_servers,
        })
    }
}
