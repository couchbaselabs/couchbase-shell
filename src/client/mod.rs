use std::collections::HashMap;

use base64::encode;
use serde::Deserialize;

pub struct OneshotClient {
    seeds: Vec<String>,
    username: String,
    password: String,
}

impl OneshotClient {
    pub fn new(seeds: Vec<String>, username: String, password: String) -> Self {
        Self {
            seeds,
            username,
            password,
        }
    }

    async fn get_config(&self) -> ClusterConfig {
        let path = "/pools/default/nodeServices";
        for seed in &self.seeds {
            let uri = format!("http://{}:8091{}", seed, &path);
            let (content, _status) = self.http_get(&uri).await;
            let mut config: ClusterConfig = serde_json::from_str(&content).unwrap();
            config.set_loaded_from(seed.clone());
            return config;
        }
        panic!()
    }

    async fn http_get(&self, uri: &str) -> (String, u16) {
        let login = encode(&format!("{}:{}", self.username, self.password));

        let mut res = surf::get(&uri)
            .header("Authorization", format!("Basic {}", login))
            .await
            .unwrap();
        let content = res.body_string().await.unwrap();
        let status = res.status() as u16;
        (content, status)
    }

    pub async fn management_request(&self, request: ManagementRequest) -> ManagementResponse {
        let config = self.get_config().await;

        let path = request.path();
        for seed in config.management_seeds() {
            let uri = format!("http://{}:{}{}", seed.0, seed.1, &path);
            let (content, status) = self.http_get(&uri).await;
            return ManagementResponse { content, status };
        }

        ManagementResponse {
            content: "".into(),
            status: 0,
        }
    }
}

pub enum ManagementRequest {
    GetBuckets,
    GetBucket { name: String },
    Whoami,
}

impl ManagementRequest {
    pub fn path(&self) -> String {
        match self {
            Self::GetBuckets => "/pools/default/buckets".into(),
            Self::GetBucket { name } => format!("/pools/default/buckets/{}", name),
            Self::Whoami => "/whoami".into(),
        }
    }
}

#[derive(Debug)]
pub struct ManagementResponse {
    content: String,
    status: u16,
}

impl ManagementResponse {
    pub fn content(&self) -> &str {
        &self.content
    }
}

#[derive(Deserialize, Debug)]
struct ClusterConfig {
    rev: u64,
    #[serde(alias = "nodesExt")]
    nodes_ext: Vec<NodeConfig>,
    loaded_from: Option<String>,
}

impl ClusterConfig {
    pub fn management_seeds(&self) -> Vec<(String, u32)> {
        self.nodes_ext
            .iter()
            .filter(|node| node.services.contains_key("mgmt"))
            .map(|node| {
                let hostname = if node.hostname.is_some() {
                    node.hostname.as_ref().unwrap().clone()
                } else {
                    self.loaded_from.as_ref().unwrap().clone()
                };
                (hostname, node.services.get("mgmt").unwrap().clone())
            })
            .collect()
    }

    pub fn set_loaded_from(&mut self, loaded_from: String) {
        self.loaded_from = Some(loaded_from);
    }
}

#[derive(Deserialize, Debug)]
struct NodeConfig {
    services: HashMap<String, u32>,
    #[serde(alias = "thisNode")]
    this_node: Option<bool>,
    hostname: Option<String>,
}
