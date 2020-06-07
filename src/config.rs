use log::debug;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

/// Holds the complete config in an aggregated manner.
#[derive(Debug, Deserialize)]
pub struct ShellConfig {
    version: usize,
    clusters: BTreeMap<String, ClusterConfig>,
}

impl ShellConfig {
    /// Tries to load the configuration from different paths.
    ///
    /// It first tries the `.cbsh/config` in the current directory, and if not found there
    /// it then tries the home directory (so `~/.cbsh/config`).
    pub fn new() -> Self {
        try_config_from_path(std::env::current_dir().unwrap())
            .or_else(|| try_config_from_path(dirs::home_dir().unwrap()))
            .unwrap_or(ShellConfig::default())
    }

    /// Builds the config from a raw input string.
    pub fn from_str(input: &str) -> Self {
        toml::from_str(input).unwrap()
    }

    /// Returns the individual configurations for all the clusters configured.
    pub fn clusters(&self) -> &BTreeMap<String, ClusterConfig> {
        &self.clusters
    }
}

impl Default for ShellConfig {
    fn default() -> Self {
        Self {
            clusters: BTreeMap::new(),
            version: 1,
        }
    }
}

fn try_config_from_path(mut path: PathBuf) -> Option<ShellConfig> {
    path.push(".cbsh");
    path.push("config");

    let read = fs::read_to_string(&path);
    match read {
        Ok(r) => Some(ShellConfig::from_str(&r)),
        Err(e) => {
            debug!("Could not locate {:?} becaue of {:?}", path, e);
            None
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ClusterConfig {
    hostnames: Vec<String>,
    #[serde(rename(deserialize = "default-bucket"))]
    default_bucket: Option<String>,

    #[serde(flatten)]
    credentials: ClusterCredentials,
}

impl ClusterConfig {
    pub fn hostnames(&self) -> &Vec<String> {
        &self.hostnames
    }
    pub fn username(&self) -> &str {
        self.credentials.username.as_str()
    }
    pub fn password(&self) -> &str {
        self.credentials.password.as_str()
    }
    pub fn cert_path(&self) -> &Option<String> {
        &self.credentials.cert_path
    }
    pub fn default_bucket(&self) -> Option<String> {
        self.default_bucket.as_ref().map(|s| s.clone())
    }
}

#[derive(Debug, Deserialize)]
pub struct ClusterCredentials {
    username: String,
    password: String,
    #[serde(rename(deserialize = "cert-path"))]
    cert_path: Option<String>,
}
