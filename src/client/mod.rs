mod codec;
mod kv;
mod protocol;

use std::collections::HashMap;

use crate::client::kv::KvEndpoint;
use crate::client::protocol::Status;
use crate::client::ClientError::CollectionNotFound;
use crate::config::ClusterTlsConfig;
use crc::crc32;
use isahc::{
    auth::{Authentication, Credentials},
    config::CaCertificate,
};
use isahc::{config::SslOption, prelude::*};
use log::kv::Source;
use nu_errors::ShellError;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fmt;
use std::ops::Deref;
use tokio::runtime::Runtime;

pub struct Client {
    seeds: Vec<String>,
    username: String,
    password: String,
    tls_config: ClusterTlsConfig,
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Clone, Serialize, Deserialize, Hash)]
pub enum ClientError {
    ConfigurationLoadFailed,
    CollectionManifestLoadFailed,
    CollectionNotFound,
    ScopeNotFound,
    KeyNotFound,
    KeyAlreadyExists,
    AccessError,
    AuthError,
    RequestFailed { reason: Option<String> },
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let message = match self {
            Self::ConfigurationLoadFailed => "failed to load config from cluster",
            Self::CollectionManifestLoadFailed => "failed to load collection manifest",
            Self::CollectionNotFound => "collection not found",
            Self::ScopeNotFound => "scope not found",
            Self::KeyNotFound => "key not found",
            Self::KeyAlreadyExists => "key already exists",
            Self::AccessError => "access error",
            Self::AuthError => "authentication error",
            Self::RequestFailed { reason } => {
                let r = reason.as_ref().unwrap();
                r.as_str()
            }
        };
        write!(f, "{}", message)
    }
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

    fn get_bucket_config(&self, bucket: String) -> Result<BucketConfig, ClientError> {
        let path = format!("/pools/default/b/{}", bucket);
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
            let mut config: BucketConfig = serde_json::from_str(&content).unwrap();
            config.set_loaded_from(seed.clone());
            return Ok(config);
        }
        Err(ClientError::ConfigurationLoadFailed)
    }

    fn get_collection_manifest(&self, bucket: String) -> Result<CollectionManifest, ClientError> {
        let path = format!("/pools/default/buckets/{}/scopes/", bucket);
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
            let manifest: CollectionManifest = serde_json::from_str(&content).unwrap();
            return Ok(manifest);
        }
        Err(ClientError::CollectionManifestLoadFailed)
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

    fn http_put(
        &self,
        uri: &str,
        payload: Option<Vec<u8>>,
        headers: HashMap<&str, &str>,
    ) -> Result<(String, u16), ClientError> {
        let res_builder = isahc::Request::put(uri);
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
                HttpVerb::Put => self.http_put(&uri, request.payload(), request.headers())?,
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

    pub fn search_query_request(
        &self,
        request: SearchQueryRequest,
    ) -> Result<HttpResponse, ClientError> {
        let config = self.get_config()?;

        let path = request.path();
        for seed in config.search_seeds(self.tls_config.enabled()) {
            let uri = format!("{}://{}:{}{}", self.http_prefix(), seed.0, seed.1, &path);
            let (content, status) = match request.verb() {
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

    fn search_manifest(
        &self,
        scope: String,
        collection: String,
        manifest: CollectionManifest,
    ) -> Result<u32, ClientError> {
        for s in manifest.scopes {
            if s.name == scope {
                for c in s.collections {
                    if c.name == collection {
                        return Ok(c.uid.parse::<u32>().unwrap());
                    }
                }
            }
        }
        return Err(CollectionNotFound);
    }

    pub fn key_value_request(
        &self,
        username: String,
        password: String,
        bucket: String,
        scope: String,
        collection: String,
        request: KeyValueRequest,
    ) -> Result<KvResponse, ClientError> {
        let config = self.get_bucket_config(bucket.clone())?;
        let mut cid: u32 = 0;
        if (scope != "" && scope != "_default") || (collection != "" && collection != "_default") {
            let manifest = match self.get_collection_manifest(bucket.clone()) {
                Ok(m) => Some(m),
                Err(e) => None,
            };
            if let Some(mani) = manifest {
                cid = self.search_manifest(scope, collection, mani)?
            }
        }

        let seeds = config.key_value_seeds(self.tls_config.enabled());
        let num_partitions = config.vbucket_server_map.vbucket_map.len() as u32;

        let key = match request {
            KeyValueRequest::Get { key } => key,
            _ => "".into(),
        };
        let sum = (crc32::checksum_ieee(key.as_bytes()) >> 16) & 0x7fff;
        let partition = sum % num_partitions;
        let node = config.vbucket_server_map.vbucket_map[partition as usize][0];

        let seed = &seeds[node as usize];
        let addr = seed.0.clone();
        let port = seed.1.clone();

        let rt = Runtime::new().unwrap();
        let result = rt.block_on(async {
            let mut ep = KvEndpoint::connect(addr, port, username, password, bucket).await;

            ep.get(key, partition as u16, cid).await
        });

        match result {
            Ok(mut r) => {
                let content = if let Some(body) = r.body() {
                    match serde_json::from_slice(body.as_ref()) {
                        Ok(v) => Some(v),
                        Err(e) => {
                            return Err(ClientError::RequestFailed {
                                reason: Some(e.to_string()),
                            });
                        }
                    }
                } else {
                    None
                };
                Ok(KvResponse {
                    status: r.status(),
                    content,
                    cas: r.cas(),
                })
            }
            Err(e) => Err(e),
        }
    }
}

pub enum HttpVerb {
    Delete,
    Get,
    Post,
    Put,
}

pub enum ManagementRequest {
    BucketStats {
        name: String,
    },
    CreateBucket {
        payload: String,
    },
    CreateCollection {
        scope: String,
        bucket: String,
        payload: String,
    },
    CreateScope {
        bucket: String,
        payload: String,
    },
    DropBucket {
        name: String,
    },
    FlushBucket {
        name: String,
    },
    GetBuckets,
    GetBucket {
        name: String,
    },
    GetCollections {
        bucket: String,
    },
    GetNodes,
    GetRoles {
        permission: Option<String>,
    },
    GetScopes {
        bucket: String,
    },
    GetUser {
        username: String,
    },
    GetUsers,
    LoadSampleBucket {
        name: String,
    },
    UpdateBucket {
        name: String,
        payload: String,
    },
    UpsertUser {
        username: String,
        payload: String,
    },
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
            Self::LoadSampleBucket { .. } => "/sampleBuckets/install".into(),
            Self::UpdateBucket { name, .. } => {
                format!("/pools/default/buckets/{}", name)
            }
            Self::CreateCollection { scope, bucket, .. } => format!(
                "/pools/default/buckets/{}/scopes/{}/collections",
                bucket, scope
            ),
            Self::GetCollections { bucket } => format!("/pools/default/buckets/{}/scopes", bucket),
            Self::GetNodes => "/pools/default".into(),
            Self::GetUsers => "/settings/rbac/users/local".into(),
            Self::GetUser { username } => format!("/settings/rbac/users/local/{}", username),
            Self::GetRoles { permission } => match permission {
                Some(p) => format!("/settings/rbac/roles?permission={}", p),
                None => "/settings/rbac/roles".into(),
            },
            Self::UpsertUser { username, .. } => format!("/settings/rbac/users/local/{}", username),
            Self::CreateScope { bucket, .. } => {
                format!("/pools/default/buckets/{}/scopes/", bucket)
            }
            Self::GetScopes { bucket } => format!("/pools/default/buckets/{}/scopes", bucket),
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
            Self::CreateCollection { .. } => HttpVerb::Post,
            Self::GetCollections { .. } => HttpVerb::Get,
            Self::GetUsers => HttpVerb::Get,
            Self::GetUser { .. } => HttpVerb::Get,
            Self::GetRoles { .. } => HttpVerb::Get,
            Self::UpsertUser { .. } => HttpVerb::Put,
            Self::GetNodes => HttpVerb::Get,
            Self::CreateScope { .. } => HttpVerb::Post,
            Self::GetScopes { .. } => HttpVerb::Get,
        }
    }

    pub fn payload(&self) -> Option<Vec<u8>> {
        match self {
            Self::CreateBucket { payload } => Some(payload.as_bytes().into()),
            Self::LoadSampleBucket { name } => Some(name.as_bytes().into()),
            Self::UpdateBucket { payload, .. } => Some(payload.as_bytes().into()),
            Self::CreateCollection { payload, .. } => Some(payload.as_bytes().into()),
            Self::UpsertUser { payload, .. } => Some(payload.as_bytes().into()),
            Self::CreateScope { payload, .. } => Some(payload.as_bytes().into()),
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
            Self::UpdateBucket { .. } => {
                let mut h = HashMap::new();
                h.insert("Content-Type", "application/x-www-form-urlencoded");
                h
            }
            Self::CreateCollection { .. } => {
                let mut h = HashMap::new();
                h.insert("Content-Type", "application/x-www-form-urlencoded");
                h
            }
            Self::UpsertUser { .. } => {
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

pub enum SearchQueryRequest {
    Execute { index: String, query: String },
}

impl SearchQueryRequest {
    pub fn path(&self) -> String {
        match self {
            Self::Execute { index, .. } => format!("/api/index/{}/query", index),
        }
    }

    pub fn verb(&self) -> HttpVerb {
        match self {
            Self::Execute { .. } => HttpVerb::Post,
        }
    }

    pub fn payload(&self) -> Option<Vec<u8>> {
        match self {
            Self::Execute { query, .. } => {
                let json = json!({ "query": { "query": query }});
                Some(serde_json::to_vec(&json).unwrap())
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

#[derive(Debug)]
pub struct KvResponse {
    content: Option<serde_json::Value>,
    status: Status,
    cas: u64,
}

impl KvResponse {
    pub fn content(&mut self) -> Option<serde_json::Value> {
        self.content.take()
    }

    pub fn status(&self) -> Status {
        self.status
    }

    pub fn cas(&self) -> u64 {
        self.cas
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

    pub fn search_seeds(&self, tls: bool) -> Vec<(String, u32)> {
        let key = if tls { "ftsSSL" } else { "fts" };

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
struct CollectionManifestCollection {
    uid: String,
    name: String,
    #[serde(alias = "maxTTL")]
    max_ttl: Option<u32>,
}

#[derive(Deserialize, Debug)]
struct CollectionManifestScope {
    uid: String,
    name: String,
    collections: Vec<CollectionManifestCollection>,
}

#[derive(Deserialize, Debug)]
struct CollectionManifest {
    uid: String,
    scopes: Vec<CollectionManifestScope>,
}

#[derive(Deserialize, Debug)]
struct BucketConfig {
    rev: u64,
    #[serde(alias = "nodesExt")]
    nodes_ext: Vec<NodeConfig>,
    loaded_from: Option<String>,
    #[serde(alias = "vBucketServerMap")]
    vbucket_server_map: VBucketServerMap,
}

impl BucketConfig {
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

    pub fn search_seeds(&self, tls: bool) -> Vec<(String, u32)> {
        let key = if tls { "ftsSSL" } else { "fts" };

        self.seeds(key)
    }

    pub fn key_value_seeds(&self, tls: bool) -> Vec<(String, u32)> {
        let key = if tls { "kvSSL" } else { "kv" };

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

#[derive(Deserialize, Debug)]
struct VBucketServerMap {
    #[serde(alias = "numReplicas")]
    num_replicas: u32,
    #[serde(alias = "serverList")]
    server_list: Vec<String>,
    #[serde(alias = "vBucketMap")]
    vbucket_map: Vec<Vec<i32>>,
}

pub enum KeyValueRequest {
    Get { key: String },
    Set { key: String, value: Option<Vec<u8>> },
}
