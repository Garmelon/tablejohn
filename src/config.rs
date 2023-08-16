//! Configuration from a file.

use std::{collections::HashMap, fs, net::SocketAddr, path::PathBuf, time::Duration};

use directories::ProjectDirs;
use serde::Deserialize;
use tracing::{debug, info};

use crate::{
    args::{Args, Command},
    id, somehow,
};

#[derive(Debug, Deserialize)]
#[serde(default)]
struct RawServerRepo {
    name: Option<String>,
    update: Duration,
    fetch_refs: Vec<String>,
    fetch_url: Option<String>,
}

impl Default for RawServerRepo {
    fn default() -> Self {
        Self {
            name: None,
            update: Duration::from_secs(60),
            fetch_refs: vec!["+refs/*:refs/*".to_string()],
            fetch_url: None,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct RawServerWeb {
    address: SocketAddr,
    base: String,
}

impl Default for RawServerWeb {
    fn default() -> Self {
        Self {
            address: "[::1]:8221".parse().unwrap(), // Port chosen by fair dice roll
            base: "/".to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct RawServerWorker {
    token: Option<String>,
    timeout: Duration,
    upload: usize,
}

impl Default for RawServerWorker {
    fn default() -> Self {
        Self {
            token: None,
            timeout: Duration::from_secs(60),
            upload: 1024 * 1024 * 8,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct RawServer {
    repo: RawServerRepo,
    web: RawServerWeb,
    worker: RawServerWorker,
}

#[derive(Debug, Deserialize)]
struct RawWorkerServer {
    url: String,
    token: String,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct RawWorker {
    name: Option<String>,
    ping: Duration,
    batch: Duration,
    servers: HashMap<String, RawWorkerServer>,
}

impl Default for RawWorker {
    fn default() -> Self {
        Self {
            name: None,
            ping: Duration::from_secs(10),
            batch: Duration::from_secs(60),
            servers: HashMap::new(),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct RawConfig {
    server: RawServer,
    worker: RawWorker,
}

#[derive(Debug)]
pub struct ServerConfig {
    pub repo_name: String,
    pub repo_update: Duration,
    pub repo_fetch_refs: Vec<String>,
    pub repo_fetch_url: Option<String>,
    pub web_address: SocketAddr,
    /// Always ends without a `/`.
    ///
    /// This means that you can prefix the base onto an absolute path and get
    /// another absolute path. You could also use an url here if you have a
    /// weird reason to do so.
    pub web_base: String,
    pub worker_token: String,
    pub worker_timeout: Duration,
    pub worker_upload: usize,
}

impl ServerConfig {
    fn repo_name(args: &Args) -> String {
        if let Command::Server(cmd) = &args.command {
            if let Some(path) = &cmd.repo {
                if let Ok(path) = path.canonicalize() {
                    if let Some(name) = path.file_name() {
                        let name = name.to_string_lossy();
                        let name = name.strip_suffix(".git").unwrap_or(&name).to_string();
                        return name;
                    }
                }
            }
        }

        "unnamed repo".to_string()
    }

    fn from_raw_server(raw: RawServer, args: &Args) -> Self {
        let repo_name = match raw.repo.name {
            Some(name) => name,
            None => Self::repo_name(args),
        };

        let web_base = raw
            .web
            .base
            .strip_suffix('/')
            .unwrap_or(&raw.web.base)
            .to_string();

        let worker_token = match raw.worker.token {
            Some(token) => token,
            None => id::random_worker_token(),
        };

        Self {
            repo_name,
            repo_update: raw.repo.update,
            repo_fetch_refs: raw.repo.fetch_refs,
            repo_fetch_url: raw.repo.fetch_url,
            web_address: raw.web.address,
            web_base,
            worker_token,
            worker_timeout: raw.worker.timeout,
            worker_upload: raw.worker.upload,
        }
    }
}

#[derive(Debug)]
pub struct WorkerServerConfig {
    /// Always ends without a `/`.
    ///
    /// This means that you can prefix the url onto an absolute path and get a
    /// correct url.
    pub url: String,
    pub token: String,
}

impl WorkerServerConfig {
    fn from_raw_worker_server(raw: RawWorkerServer) -> Self {
        Self {
            url: raw.url.strip_suffix('/').unwrap_or(&raw.url).to_string(),
            token: raw.token,
        }
    }
}

#[derive(Debug)]
pub struct WorkerConfig {
    pub name: String,
    pub ping: Duration,
    pub batch: Duration,
    pub servers: HashMap<String, WorkerServerConfig>,
}

impl WorkerConfig {
    fn from_raw_worker(raw: RawWorker) -> Self {
        let name = match raw.name {
            Some(name) => name,
            None => gethostname::gethostname().to_string_lossy().to_string(),
        };

        let servers = raw
            .servers
            .into_iter()
            .map(|(k, v)| (k, WorkerServerConfig::from_raw_worker_server(v)))
            .collect();

        Self {
            name,
            ping: raw.ping,
            batch: raw.batch,
            servers,
        }
    }
}

#[derive(Debug)]
pub struct Config {
    pub server: ServerConfig,
    pub worker: WorkerConfig,
}

impl Config {
    fn from_raw_config(raw: RawConfig, args: &Args) -> Self {
        Self {
            server: ServerConfig::from_raw_server(raw.server, args),
            worker: WorkerConfig::from_raw_worker(raw.worker),
        }
    }

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

        let raw = fs::read_to_string(path)?;
        let raw = toml::from_str::<RawConfig>(&raw)?;
        debug!("Loaded raw config: {raw:#?}");
        let config = Self::from_raw_config(raw, args);
        debug!("Loaded config: {config:#?}");
        Ok(config)
    }
}
