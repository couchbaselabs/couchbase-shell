use std::collections::BTreeMap;
use serde::Deserialize;
use std::fs;
use log::debug;

#[derive(Debug, Deserialize)]
pub struct ShellConfig {
    clusters: BTreeMap<String, ClusterConfig>,
}

impl ShellConfig {

    pub fn new() -> Self {
        let read = fs::read_to_string(".cbshrc");
        match read {
            Ok(s) => Self::from_str(&s),
            Err(e) => {
                debug!("Could not locate .cbshrc becaue of {:?}", e);
                ShellConfig {
                    clusters: BTreeMap::new(),
                }
            }
        }
    }

    pub fn from_str(input: &str) -> Self {
        toml::from_str(input).unwrap()
    }

    pub fn clusters(&self) -> &BTreeMap<String, ClusterConfig> {
        &self.clusters
    }
}

#[derive(Debug, Deserialize)]
pub struct ClusterConfig {
    connstr: String,
    username: String,
    password: String,
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
}

