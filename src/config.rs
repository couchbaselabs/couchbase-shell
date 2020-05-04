use log::debug;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct ShellConfig {
    clusters: BTreeMap<String, ClusterConfig>,
}

impl ShellConfig {
    pub fn new() -> Self {
        // first, try current dir
        if let Some(c) = try_config_from_path(std::env::current_dir().unwrap()) {
            return c;
        }

        // then, try home dir
        if let Some(c) = try_config_from_path(dirs::home_dir().unwrap()) {
            return c;
        }

        // if both are not found, return an empty config
        ShellConfig {
            clusters: BTreeMap::new(),
        }
    }

    pub fn from_str(input: &str) -> Self {
        toml::from_str(input).unwrap()
    }

    pub fn clusters(&self) -> &BTreeMap<String, ClusterConfig> {
        &self.clusters
    }
}

fn try_config_from_path(mut path: PathBuf) -> Option<ShellConfig> {
    path.push(".cbsh");
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
    connstr: String,
    username: String,
    password: String,
    #[serde(rename(deserialize = "default-bucket"))]
    default_bucket: Option<String>,
}

impl ClusterConfig {
    pub fn connstr(&self) -> &str {
        self.connstr.as_str()
    }
    pub fn username(&self) -> &str {
        self.username.as_str()
    }
    pub fn password(&self) -> &str {
        self.password.as_str()
    }
    pub fn default_bucket(&self) -> Option<String> {
        self.default_bucket.as_ref().map(|s| s.clone())
    }
}
