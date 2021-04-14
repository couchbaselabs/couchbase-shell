use std::collections::HashMap;

use base64::encode;
use nu_errors::ShellError;
use serde::{Deserialize, Serialize};
use serde_json::json;

pub struct Client {
    seeds: Vec<String>,
    username: String,
    password: String,
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Clone, Serialize, Deserialize, Hash)]
pub enum ClientError {
    ConfigurationLoadFailed,
    RequestFailed,
}

impl From<ClientError> for ShellError {
    fn from(ce: ClientError) -> Self {
        // todo: this can definitely be improved with more detail and reporting specifics
        ShellError::untagged_runtime_error(serde_json::to_string(&ce).unwrap())
    }
}

impl Client {
    pub fn new(seeds: Vec<String>, username: String, password: String) -> Self {
        Self {
            seeds,
            username,
            password,
        }
    }

    async fn get_config(&self) -> Result<ClusterConfig, ClientError> {
        let path = "/pools/default/nodeServices";
        for seed in &self.seeds {
            let uri = format!("http://{}:8091{}", seed, &path);
            let (content, status) = self.http_get(&uri).await?;
            if status != 200 {
                continue;
            }
            let mut config: ClusterConfig = serde_json::from_str(&content).unwrap();
            config.set_loaded_from(seed.clone());
            return Ok(config);
        }
        Err(ClientError::ConfigurationLoadFailed)
    }

    async fn http_get(&self, uri: &str) -> Result<(String, u16), ClientError> {
        let login = encode(&format!("{}:{}", self.username, self.password));

        let mut res = surf::get(&uri)
            .header("Authorization", format!("Basic {}", login))
            .await
            .unwrap();
        let content = res.body_string().await.unwrap();
        let status = res.status() as u16;
        Ok((content, status))
    }

    async fn http_post(
        &self,
        uri: &str,
        payload: Option<http_types::Body>,
    ) -> Result<(String, u16), ClientError> {
        let login = encode(&format!("{}:{}", self.username, self.password));

        let mut res = surf::post(&uri)
            .body(payload.unwrap())
            .header("Authorization", format!("Basic {}", login))
            .await
            .unwrap();
        let content = res.body_string().await.unwrap();
        let status = res.status() as u16;
        Ok((content, status))
    }

    pub async fn management_request(
        &self,
        request: ManagementRequest,
    ) -> Result<HttpResponse, ClientError> {
        let config = self.get_config().await?;

        let path = request.path();
        for seed in config.management_seeds() {
            let uri = format!("http://{}:{}{}", seed.0, seed.1, &path);
            let (content, status) = self.http_get(&uri).await?;
            return Ok(HttpResponse { content, status });
        }

        Err(ClientError::RequestFailed)
    }

    pub async fn query_request(&self, request: QueryRequest) -> Result<HttpResponse, ClientError> {
        let config = self.get_config().await?;

        let path = request.path();
        for seed in config.query_seeds() {
            let uri = format!("http://{}:{}{}", seed.0, seed.1, &path);
            let (content, status) = match request.verb() {
                HttpVerb::Get => self.http_get(&uri).await?,
                HttpVerb::Post => self.http_post(&uri, request.payload()).await?,
            };

            return Ok(HttpResponse { content, status });
        }

        Err(ClientError::RequestFailed)
    }
}

pub enum HttpVerb {
    Get,
    Post,
}

pub enum ManagementRequest {
    GetBuckets,
    GetBucket { name: String },
    Whoami,
    IndexStatus,
}

impl ManagementRequest {
    pub fn path(&self) -> String {
        match self {
            Self::GetBuckets => "/pools/default/buckets".into(),
            Self::GetBucket { name } => format!("/pools/default/buckets/{}", name),
            Self::Whoami => "/whoami".into(),
            Self::IndexStatus => "/indexStatus".into(),
        }
    }

    pub fn verb(&self) -> HttpVerb {
        match self {
            Self::GetBuckets => HttpVerb::Get,
            Self::GetBucket { .. } => HttpVerb::Get,
            Self::Whoami => HttpVerb::Get,
            Self::IndexStatus => HttpVerb::Get,
        }
    }

    pub fn payload(&self) -> Option<http_types::Body> {
        None
    }
}

pub enum QueryRequest {
    Execute {
        statement: String,
        scope: Option<(String, String)>,
    },
}

impl QueryRequest {
    pub fn path(&self) -> String {
        match self {
            Self::Execute { .. } => "/query".into(),
        }
    }

    pub fn verb(&self) -> HttpVerb {
        match self {
            Self::Execute { .. } => HttpVerb::Post,
        }
    }

    pub fn payload(&self) -> Option<http_types::Body> {
        match self {
            Self::Execute { statement, scope } => {
                if let Some(scope) = scope {
                    let ctx = format!("`default`:`{}`.`{}", scope.0, scope.1);
                    Some(
                        http_types::Body::from_json(
                            &json!({ "statement": statement, "query_context": ctx }),
                        )
                        .unwrap(),
                    )
                } else {
                    Some(http_types::Body::from_json(&json!({ "statement": statement })).unwrap())
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct HttpResponse {
    content: String,
    status: u16,
}

impl HttpResponse {
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

    pub fn query_seeds(&self) -> Vec<(String, u32)> {
        self.nodes_ext
            .iter()
            .filter(|node| node.services.contains_key("n1ql"))
            .map(|node| {
                let hostname = if node.hostname.is_some() {
                    node.hostname.as_ref().unwrap().clone()
                } else {
                    self.loaded_from.as_ref().unwrap().clone()
                };
                (hostname, node.services.get("n1ql").unwrap().clone())
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
