use std::collections::HashMap;

use crate::config::ClusterTlsConfig;
use isahc::{
    auth::{Authentication, Credentials},
    config::CaCertificate,
};
use isahc::{config::SslOption, prelude::*};
use nu_errors::ShellError;
use serde::{Deserialize, Serialize};
use serde_json::json;

pub struct Client {
    seeds: Vec<String>,
    username: String,
    password: String,
    tls_config: ClusterTlsConfig,
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Clone, Serialize, Deserialize, Hash)]
pub enum ClientError {
    ConfigurationLoadFailed,
    RequestFailed { reason: Option<String> },
}

impl From<ClientError> for ShellError {
    fn from(ce: ClientError) -> Self {
        // todo: this can definitely be improved with more detail and reporting specifics
        ShellError::untagged_runtime_error(serde_json::to_string(&ce).unwrap())
    }
}

impl From<std::io::Error> for ClientError {
    fn from(e: std::io::Error) -> Self {
        ClientError::RequestFailed {
            reason: Some(format!("{}", e)),
        }
    }
}

impl From<isahc::Error> for ClientError {
    fn from(e: isahc::Error) -> Self {
        ClientError::RequestFailed {
            reason: Some(format!("{}", e)),
        }
    }
}

impl From<isahc::http::Error> for ClientError {
    fn from(e: isahc::http::Error) -> Self {
        ClientError::RequestFailed {
            reason: Some(format!("{}", e)),
        }
    }
}

impl Client {
    pub fn new(
        seeds: Vec<String>,
        username: String,
        password: String,
        tls_config: ClusterTlsConfig,
    ) -> Self {
        Self {
            seeds,
            username,
            password,
            tls_config: tls_config.clone(),
        }
    }

    fn http_prefix(&self) -> &'static str {
        match self.tls_config.enabled() {
            true => "https",
            false => "http",
        }
    }

    fn get_config(&self) -> Result<ClusterConfig, ClientError> {
        let path = "/pools/default/nodeServices";
        let port = if self.tls_config.enabled() {
            18091
        } else {
            8091
        };
        for seed in &self.seeds {
            let uri = format!("{}://{}:{}{}", self.http_prefix(), seed, port, &path);
            let (content, status) = self.http_get(&uri)?;
            if status != 200 {
                continue;
            }
            let mut config: ClusterConfig = serde_json::from_str(&content).unwrap();
            config.set_loaded_from(seed.clone());
            return Ok(config);
        }
        Err(ClientError::ConfigurationLoadFailed)
    }

    fn http_do(
        &self,
        mut res_builder: http::request::Builder,
        payload: Option<Vec<u8>>,
        headers: HashMap<&str, &str>,
    ) -> Result<(String, u16), ClientError> {
        res_builder = res_builder
            .authentication(Authentication::basic())
            .credentials(Credentials::new(&self.username, &self.password));

        if self.tls_config.enabled() {
            if let Some(cert) = self.tls_config.cert_path() {
                res_builder = res_builder.ssl_ca_certificate(CaCertificate::file(cert));
            }
            res_builder = res_builder.ssl_options(self.http_ssl_opts());
        }

        for (key, value) in headers {
            res_builder = res_builder.header(key, value);
        }

        let mut res: http::Response<isahc::Body>;
        if let Some(p) = payload {
            res = res_builder.body(p)?.send()?;
        } else {
            res = res_builder.body(())?.send()?;
        }

        let content = res.text()?;
        let status = res.status().into();
        Ok((content, status))
    }

    fn http_get(&self, uri: &str) -> Result<(String, u16), ClientError> {
        let res_builder = isahc::Request::get(uri);
        self.http_do(res_builder, None, HashMap::new())
    }

    fn http_delete(&self, uri: &str) -> Result<(String, u16), ClientError> {
        let res_builder = isahc::Request::delete(uri);
        self.http_do(res_builder, None, HashMap::new())
    }

    fn http_ssl_opts(&self) -> SslOption {
        let mut ssl_opts = SslOption::NONE;
        if !self.tls_config.validate_hostnames() {
            ssl_opts = ssl_opts | SslOption::DANGER_ACCEPT_INVALID_HOSTS;
        }
        if self.tls_config.accept_all_certs() {
            ssl_opts = ssl_opts | SslOption::DANGER_ACCEPT_INVALID_CERTS;
        }
        ssl_opts
    }

    fn http_post(
        &self,
        uri: &str,
        payload: Option<Vec<u8>>,
        headers: HashMap<&str, &str>,
    ) -> Result<(String, u16), ClientError> {
        let res_builder = isahc::Request::post(uri);
        self.http_do(res_builder, payload, headers)
    }

    pub fn management_request(
        &self,
        request: ManagementRequest,
    ) -> Result<HttpResponse, ClientError> {
        let config = self.get_config()?;

        let path = request.path();
        for seed in config.management_seeds(self.tls_config.enabled()) {
            let uri = format!("{}://{}:{}{}", self.http_prefix(), seed.0, seed.1, &path);
            let (content, status) = match request.verb() {
                HttpVerb::Get => self.http_get(&uri)?,
                HttpVerb::Post => self.http_post(&uri, request.payload(), request.headers())?,
                HttpVerb::Delete => self.http_delete(&uri)?,
            };
            return Ok(HttpResponse { content, status });
        }

        Err(ClientError::RequestFailed { reason: None })
    }

    pub fn query_request(&self, request: QueryRequest) -> Result<HttpResponse, ClientError> {
        let config = self.get_config()?;

        let path = request.path();
        for seed in config.query_seeds(self.tls_config.enabled()) {
            let uri = format!("{}://{}:{}{}", self.http_prefix(), seed.0, seed.1, &path);
            let (content, status) = match request.verb() {
                HttpVerb::Get => self.http_get(&uri)?,
                HttpVerb::Post => self.http_post(&uri, request.payload(), request.headers())?,
                _ => {
                    return Err(ClientError::RequestFailed {
                        reason: Some("Method not allowed for queries".into()),
                    });
                }
            };

            return Ok(HttpResponse { content, status });
        }

        Err(ClientError::RequestFailed { reason: None })
    }

    pub fn analytics_query_request(
        &self,
        request: AnalyticsQueryRequest,
    ) -> Result<HttpResponse, ClientError> {
        let config = self.get_config()?;

        let path = request.path();
        for seed in config.analytics_seeds(self.tls_config.enabled()) {
            let uri = format!("{}://{}:{}{}", self.http_prefix(), seed.0, seed.1, &path);
            let (content, status) = match request.verb() {
                HttpVerb::Get => self.http_get(&uri)?,
                HttpVerb::Post => self.http_post(&uri, request.payload(), request.headers())?,
                _ => {
                    return Err(ClientError::RequestFailed {
                        reason: Some("Method not allowed for analytics queries".into()),
                    });
                }
            };

            return Ok(HttpResponse { content, status });
        }

        Err(ClientError::RequestFailed { reason: None })
    }
}

pub enum HttpVerb {
    Get,
    Post,
    Delete,
}

pub enum ManagementRequest {
    BucketStats { name: String },
    CreateBucket { payload: String },
    DropBucket { name: String },
    FlushBucket { name: String },
    GetBuckets,
    GetBucket { name: String },
    LoadSampleBucket { name: String },
    UpdateBucket { name: String, payload: String },
    IndexStatus,
    SettingsAutoFailover,
    Whoami,
}

impl ManagementRequest {
    pub fn path(&self) -> String {
        match self {
            Self::GetBuckets => "/pools/default/buckets".into(),
            Self::GetBucket { name } => format!("/pools/default/buckets/{}", name),
            Self::Whoami => "/whoami".into(),
            Self::IndexStatus => "/indexStatus".into(),
            Self::SettingsAutoFailover => "/settings/autoFailover".into(),
            Self::BucketStats { name } => format!("/pools/default/buckets/{}/stats", name),
            Self::CreateBucket { .. } => "/pools/default/buckets".into(),
            Self::DropBucket { name } => format!("/pools/default/buckets/{}", name),
            Self::FlushBucket { name } => {
                format!("/pools/default/buckets/{}/controller/doFlush", name)
            }
            Self::LoadSampleBucket { name } => "/sampleBuckets/install".into(),
            Self::UpdateBucket { name, .. } => {
                format!("/pools/default/buckets/{}", name)
            }
        }
    }

    pub fn verb(&self) -> HttpVerb {
        match self {
            Self::GetBuckets => HttpVerb::Get,
            Self::GetBucket { .. } => HttpVerb::Get,
            Self::Whoami => HttpVerb::Get,
            Self::IndexStatus => HttpVerb::Get,
            Self::SettingsAutoFailover => HttpVerb::Get,
            Self::BucketStats { .. } => HttpVerb::Get,
            Self::CreateBucket { .. } => HttpVerb::Post,
            Self::DropBucket { .. } => HttpVerb::Delete,
            Self::FlushBucket { .. } => HttpVerb::Post,
            Self::LoadSampleBucket { .. } => HttpVerb::Post,
            Self::UpdateBucket { .. } => HttpVerb::Post,
        }
    }

    pub fn payload(&self) -> Option<Vec<u8>> {
        match self {
            Self::CreateBucket { payload } => Some(payload.as_bytes().into()),
            Self::LoadSampleBucket { name } => Some(name.as_bytes().into()),
            Self::UpdateBucket { payload, .. } => Some(payload.as_bytes().into()),
            _ => None,
        }
    }

    pub fn headers(&self) -> HashMap<&str, &str> {
        match self {
            Self::CreateBucket { .. } => {
                let mut h = HashMap::new();
                h.insert("Content-Type", "application/x-www-form-urlencoded");
                h
            }
            _ => HashMap::new(),
        }
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

    pub fn payload(&self) -> Option<Vec<u8>> {
        match self {
            Self::Execute { statement, scope } => {
                if let Some(scope) = scope {
                    let ctx = format!("`default`:`{}`.`{}`", scope.0, scope.1);
                    let json = json!({ "statement": statement, "query_context": ctx });
                    Some(serde_json::to_vec(&json).unwrap())
                } else {
                    let json = json!({ "statement": statement });
                    Some(serde_json::to_vec(&json).unwrap())
                }
            }
        }
    }

    pub fn headers(&self) -> HashMap<&str, &str> {
        match self {
            Self::Execute { .. } => {
                let mut h = HashMap::new();
                h.insert("Content-Type", "application/json");
                h
            }
            _ => HashMap::new(),
        }
    }
}

pub enum AnalyticsQueryRequest {
    Execute {
        statement: String,
        scope: Option<(String, String)>,
    },
}

impl AnalyticsQueryRequest {
    pub fn path(&self) -> String {
        match self {
            Self::Execute { .. } => "/query/service".into(),
        }
    }

    pub fn verb(&self) -> HttpVerb {
        match self {
            Self::Execute { .. } => HttpVerb::Post,
        }
    }

    pub fn payload(&self) -> Option<Vec<u8>> {
        match self {
            Self::Execute { statement, scope } => {
                if let Some(scope) = scope {
                    let ctx = format!("`default`:`{}`.`{}`", scope.0, scope.1);
                    let json = json!({ "statement": statement, "query_context": ctx });
                    Some(serde_json::to_vec(&json).unwrap())
                } else {
                    let json = json!({ "statement": statement });
                    Some(serde_json::to_vec(&json).unwrap())
                }
            }
        }
    }

    pub fn headers(&self) -> HashMap<&str, &str> {
        match self {
            Self::Execute { .. } => {
                let mut h = HashMap::new();
                h.insert("Content-Type", "application/json");
                h
            }
            _ => HashMap::new(),
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

    pub fn status(&self) -> u16 {
        self.status
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
    pub fn management_seeds(&self, tls: bool) -> Vec<(String, u32)> {
        let key = if tls { "mgmtSSL" } else { "mgmt" };

        self.seeds(key)
    }

    pub fn query_seeds(&self, tls: bool) -> Vec<(String, u32)> {
        let key = if tls { "n1qlSSL" } else { "n1ql" };

        self.seeds(key)
    }

    pub fn analytics_seeds(&self, tls: bool) -> Vec<(String, u32)> {
        let key = if tls { "cbasSSL" } else { "cbas" };

        self.seeds(key)
    }

    pub fn set_loaded_from(&mut self, loaded_from: String) {
        self.loaded_from = Some(loaded_from);
    }

    fn seeds(&self, key: &str) -> Vec<(String, u32)> {
        self.nodes_ext
            .iter()
            .filter(|node| node.services.contains_key(key))
            .map(|node| {
                let hostname = if node.hostname.is_some() {
                    node.hostname.as_ref().unwrap().clone()
                } else {
                    self.loaded_from.as_ref().unwrap().clone()
                };
                (hostname, node.services.get(key).unwrap().clone())
            })
            .collect()
    }
}

#[derive(Deserialize, Debug)]
struct NodeConfig {
    services: HashMap<String, u32>,
    #[serde(alias = "thisNode")]
    this_node: Option<bool>,
    hostname: Option<String>,
}