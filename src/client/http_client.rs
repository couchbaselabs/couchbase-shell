use crate::client::error::ClientError;
use crate::client::http_handler::{
    http_prefix, status_to_reason, HTTPHandler, HttpResponse, HttpVerb,
};
use crate::client::kv_client::NodeConfig;
use crate::config::ClusterTlsConfig;
use rand::Rng;
use serde::Deserialize;
use serde_json::json;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;
use std::{collections::HashMap, ops::Sub};
use tokio::runtime::Runtime;
use tokio::time::Instant;

pub struct HTTPClient {
    seeds: Vec<String>,
    tls_config: ClusterTlsConfig,
    http_client: HTTPHandler,
}

impl HTTPClient {
    pub fn new(
        seeds: Vec<String>,
        username: String,
        password: String,
        tls_config: ClusterTlsConfig,
    ) -> Self {
        Self {
            seeds,
            http_client: HTTPHandler::new(username, password, tls_config.clone()),
            tls_config,
        }
    }

    async fn get_config(
        &self,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<ClusterConfig, ClientError> {
        let path = "/pools/default/nodeServices";
        let mut final_error_content = None;
        let mut final_error_status = 0;
        for seed in &self.seeds {
            let host_split: Vec<String> = seed.split(':').map(|v| v.to_owned()).collect();

            let host: String;
            let port: i32;
            if host_split.len() == 1 {
                host = seed.clone();
                port = if self.tls_config.enabled() {
                    18091
                } else {
                    8091
                };
            } else {
                host = host_split[0].clone();
                port = host_split[1]
                    .parse::<i32>()
                    .map_err(|e| ClientError::RequestFailed {
                        reason: Some(e.to_string()),
                        key: None,
                    })?;
            }

            let uri = format!(
                "{}://{}:{}{}",
                http_prefix(&self.tls_config),
                host,
                port,
                &path
            );
            let (content, status) = self
                .http_client
                .http_get(&uri, deadline, ctrl_c.clone())
                .await?;
            if status != 200 {
                if !content.is_empty() {
                    final_error_content = Some(content);
                }
                final_error_status = status;
                continue;
            }
            let mut config: ClusterConfig = serde_json::from_str(&content).unwrap();
            config.set_loaded_from(host);
            return Ok(config);
        }
        let mut reason = final_error_content;
        if reason.is_none() {
            reason = status_to_reason(final_error_status);
        }
        Err(ClientError::ConfigurationLoadFailed { reason })
    }

    async fn ping_endpoint(
        &self,
        uri: String,
        address: String,
        service: ServiceType,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<PingResponse, ClientError> {
        let start = Instant::now();
        let result = self.http_client.http_get(&uri, deadline, ctrl_c).await;
        let end = Instant::now();

        let error = match result {
            Ok(_) => None,
            Err(e) => Some(e),
        };

        let mut state = "OK".into();
        if error.is_some() {
            state = "Error".into();
        }

        Ok(PingResponse {
            state,
            address,
            service,
            latency: end.sub(start),
            error,
        })
    }

    // TODO: parallelize this.
    pub fn ping_all_request(
        &self,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<Vec<PingResponse>, ClientError> {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let config = self.get_config(deadline, ctrl_c.clone()).await?;

            let mut results: Vec<PingResponse> = Vec::new();
            for seed in config.search_seeds(self.tls_config.enabled()) {
                let uri = format!(
                    "{}://{}:{}/api/ping",
                    http_prefix(&self.tls_config),
                    seed.0,
                    seed.1
                );
                let address = format!("{}:{}", seed.0, seed.1);
                results.push(
                    self.ping_endpoint(uri, address, ServiceType::Search, deadline, ctrl_c.clone())
                        .await?,
                );
            }
            for seed in config.query_seeds(self.tls_config.enabled()) {
                let uri = format!(
                    "{}://{}:{}/admin/ping",
                    http_prefix(&self.tls_config),
                    seed.0,
                    seed.1
                );
                let address = format!("{}:{}", seed.0, seed.1);
                results.push(
                    self.ping_endpoint(uri, address, ServiceType::Query, deadline, ctrl_c.clone())
                        .await?,
                );
            }
            for seed in config.analytics_seeds(self.tls_config.enabled()) {
                let uri = format!(
                    "{}://{}:{}/admin/ping",
                    http_prefix(&self.tls_config),
                    seed.0,
                    seed.1
                );
                let address = format!("{}:{}", seed.0, seed.1);
                results.push(
                    self.ping_endpoint(
                        uri,
                        address,
                        ServiceType::Analytics,
                        deadline,
                        ctrl_c.clone(),
                    )
                    .await?,
                );
            }
            for seed in config.view_seeds(self.tls_config.enabled()) {
                let uri = format!("{}://{}:{}/", http_prefix(&self.tls_config), seed.0, seed.1);
                let address = format!("{}:{}", seed.0, seed.1);
                results.push(
                    self.ping_endpoint(uri, address, ServiceType::Views, deadline, ctrl_c.clone())
                        .await?,
                );
            }

            Ok(results)
        })
    }

    pub fn management_request(
        &self,
        request: ManagementRequest,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<HttpResponse, ClientError> {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let config = self.get_config(deadline, ctrl_c.clone()).await?;

            let path = request.path();
            if let Some(seed) = config.random_management_seed(self.tls_config.enabled()) {
                let uri = format!(
                    "{}://{}:{}{}",
                    http_prefix(&self.tls_config),
                    seed.0,
                    seed.1,
                    &path
                );
                let (content, status) = match request.verb() {
                    HttpVerb::Get => self.http_client.http_get(&uri, deadline, ctrl_c).await?,
                    HttpVerb::Post => {
                        self.http_client
                            .http_post(&uri, request.payload(), request.headers(), deadline, ctrl_c)
                            .await?
                    }
                    HttpVerb::Delete => {
                        self.http_client.http_delete(&uri, deadline, ctrl_c).await?
                    }
                    HttpVerb::Put => {
                        self.http_client
                            .http_put(&uri, request.payload(), request.headers(), deadline, ctrl_c)
                            .await?
                    }
                };
                return Ok(HttpResponse::new(content, status));
            }

            Err(ClientError::RequestFailed {
                reason: Some("No nodes found for service".to_string()),
                key: None,
            })
        })
    }

    pub fn query_request(
        &self,
        request: QueryRequest,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<HttpResponse, ClientError> {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let config = self.get_config(deadline, ctrl_c.clone()).await?;

            let path = request.path();
            if let Some(seed) = config.random_query_seed(self.tls_config.enabled()) {
                let uri = format!(
                    "{}://{}:{}{}",
                    http_prefix(&self.tls_config),
                    seed.0,
                    seed.1,
                    &path
                );
                let (content, status) = match request.verb() {
                    HttpVerb::Get => self.http_client.http_get(&uri, deadline, ctrl_c).await?,
                    HttpVerb::Post => {
                        self.http_client
                            .http_post(&uri, request.payload(), request.headers(), deadline, ctrl_c)
                            .await?
                    }
                    _ => {
                        return Err(ClientError::RequestFailed {
                            reason: Some("Method not allowed for queries".into()),
                            key: None,
                        });
                    }
                };

                return Ok(HttpResponse::new(content, status));
            }

            Err(ClientError::RequestFailed {
                reason: Some("No nodes found for service".to_string()),
                key: None,
            })
        })
    }

    pub fn analytics_query_request(
        &self,
        request: AnalyticsQueryRequest,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<HttpResponse, ClientError> {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let config = self.get_config(deadline, ctrl_c.clone()).await?;

            let path = request.path();
            if let Some(seed) = config.random_analytics_seed(self.tls_config.enabled()) {
                let uri = format!(
                    "{}://{}:{}{}",
                    http_prefix(&self.tls_config),
                    seed.0,
                    seed.1,
                    &path
                );
                let (content, status) = match request.verb() {
                    HttpVerb::Get => self.http_client.http_get(&uri, deadline, ctrl_c).await?,
                    HttpVerb::Post => {
                        self.http_client
                            .http_post(&uri, request.payload(), request.headers(), deadline, ctrl_c)
                            .await?
                    }
                    _ => {
                        return Err(ClientError::RequestFailed {
                            reason: Some("Method not allowed for analytics queries".into()),
                            key: None,
                        });
                    }
                };

                return Ok(HttpResponse::new(content, status));
            }

            Err(ClientError::RequestFailed {
                reason: Some("No nodes found for service".to_string()),
                key: None,
            })
        })
    }

    pub fn search_query_request(
        &self,
        request: SearchQueryRequest,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<HttpResponse, ClientError> {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let config = self.get_config(deadline, ctrl_c.clone()).await?;

            let path = request.path();
            if let Some(seed) = config.random_search_seed(self.tls_config.enabled()) {
                let uri = format!(
                    "{}://{}:{}{}",
                    http_prefix(&self.tls_config),
                    seed.0,
                    seed.1,
                    &path
                );
                let (content, status) = match request.verb() {
                    HttpVerb::Post => {
                        self.http_client
                            .http_post(&uri, request.payload(), request.headers(), deadline, ctrl_c)
                            .await?
                    }
                    _ => {
                        return Err(ClientError::RequestFailed {
                            reason: Some("Method not allowed for analytics queries".into()),
                            key: None,
                        });
                    }
                };

                return Ok(HttpResponse::new(content, status));
            }

            Err(ClientError::RequestFailed {
                reason: Some("No nodes found for service".to_string()),
                key: None,
            })
        })
    }
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
    DropCollection {
        scope: String,
        bucket: String,
        name: String,
    },
    DropScope {
        name: String,
        bucket: String,
    },
    DropUser {
        username: String,
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
            Self::DropUser { username } => format!("/settings/rbac/users/local/{}", username),
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
            Self::DropCollection {
                scope,
                bucket,
                name,
            } => format!(
                "/pools/default/buckets/{}/scopes/{}/collections/{}",
                bucket, scope, name
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
            Self::DropScope { bucket, name } => {
                format!("/pools/default/buckets/{}/scopes/{}", bucket, name)
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
            Self::DropUser { .. } => HttpVerb::Delete,
            Self::FlushBucket { .. } => HttpVerb::Post,
            Self::LoadSampleBucket { .. } => HttpVerb::Post,
            Self::UpdateBucket { .. } => HttpVerb::Post,
            Self::CreateCollection { .. } => HttpVerb::Post,
            Self::DropCollection { .. } => HttpVerb::Delete,
            Self::GetCollections { .. } => HttpVerb::Get,
            Self::GetUsers => HttpVerb::Get,
            Self::GetUser { .. } => HttpVerb::Get,
            Self::GetRoles { .. } => HttpVerb::Get,
            Self::UpsertUser { .. } => HttpVerb::Put,
            Self::GetNodes => HttpVerb::Get,
            Self::CreateScope { .. } => HttpVerb::Post,
            Self::DropScope { .. } => HttpVerb::Delete,
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
        timeout: String,
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
            Self::Execute {
                statement,
                scope,
                timeout,
            } => {
                if let Some(scope) = scope {
                    let ctx = format!("`default`:`{}`.`{}`", scope.0, scope.1);
                    let json =
                        json!({ "statement": statement, "query_context": ctx, "timeout": timeout });
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
        }
    }
}

pub enum AnalyticsQueryRequest {
    Execute {
        statement: String,
        scope: Option<(String, String)>,
        timeout: String,
    },
    PendingMutations,
}

impl AnalyticsQueryRequest {
    pub fn path(&self) -> String {
        match self {
            Self::Execute { .. } => "/query/service".into(),
            Self::PendingMutations => "/analytics/node/agg/stats/remaining".into(),
        }
    }

    pub fn verb(&self) -> HttpVerb {
        match self {
            Self::Execute { .. } => HttpVerb::Post,
            Self::PendingMutations => HttpVerb::Get,
        }
    }

    pub fn payload(&self) -> Option<Vec<u8>> {
        match self {
            Self::Execute {
                statement,
                scope,
                timeout,
            } => {
                if let Some(scope) = scope {
                    let ctx = format!("`default`:`{}`.`{}`", scope.0, scope.1);
                    let json =
                        json!({ "statement": statement, "query_context": ctx, "timeout": timeout });
                    Some(serde_json::to_vec(&json).unwrap())
                } else {
                    let json = json!({ "statement": statement });
                    Some(serde_json::to_vec(&json).unwrap())
                }
            }
            Self::PendingMutations => None,
        }
    }

    pub fn headers(&self) -> HashMap<&str, &str> {
        match self {
            Self::Execute { .. } => {
                let mut h = HashMap::new();
                h.insert("Content-Type", "application/json");
                h
            }
            Self::PendingMutations => HashMap::new(),
        }
    }
}

pub enum SearchQueryRequest {
    Execute {
        index: String,
        query: String,
        timeout: String,
    },
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
            Self::Execute { query, timeout, .. } => {
                let json = json!({ "query": { "query": query }, "ctl": { "timeout": timeout }});
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
        }
    }
}

#[derive(Debug)]
pub struct PingResponse {
    state: String,
    address: String,
    service: ServiceType,
    latency: Duration,
    error: Option<ClientError>,
}

impl PingResponse {
    pub(crate) fn new(
        state: String,
        address: String,
        service: ServiceType,
        latency: Duration,
        error: Option<ClientError>,
    ) -> Self {
        Self {
            state,
            address,
            service,
            latency,
            error,
        }
    }
    pub fn state(&self) -> &str {
        &self.state
    }

    pub fn address(&self) -> &str {
        &self.address
    }

    pub fn service(&self) -> &ServiceType {
        &self.service
    }

    pub fn latency(&self) -> Duration {
        self.latency
    }

    pub fn error(&self) -> Option<&ClientError> {
        self.error.as_ref()
    }
}

#[derive(Debug)]
pub enum ServiceType {
    KeyValue,
    Query,
    Search,
    Analytics,
    Views,
}

impl ServiceType {
    pub fn as_string(&self) -> String {
        match self {
            ServiceType::KeyValue => "KeyValue".into(),
            ServiceType::Query => "Query".into(),
            ServiceType::Search => "Search".into(),
            ServiceType::Analytics => "Analytics".into(),
            ServiceType::Views => "Views".into(),
        }
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

    pub fn view_seeds(&self, tls: bool) -> Vec<(String, u32)> {
        let key = if tls { "capiSSL" } else { "capi" };

        self.seeds(key)
    }

    pub fn set_loaded_from(&mut self, loaded_from: String) {
        self.loaded_from = Some(loaded_from);
    }

    fn seeds(&self, key: &str) -> Vec<(String, u32)> {
        let default: Vec<(String, u32)> = self
            .nodes_ext
            .iter()
            .filter(|node| node.services.contains_key(key))
            .map(|node| {
                let hostname = if node.hostname.is_some() {
                    node.hostname.as_ref().unwrap().clone()
                } else {
                    self.loaded_from.as_ref().unwrap().clone()
                };
                (hostname, *node.services.get(key).unwrap())
            })
            .collect();

        for seed in &default {
            if seed.0 == self.loaded_from.as_ref().unwrap().clone() {
                return default;
            }
        }

        let external: Vec<(String, u32)> = self
            .nodes_ext
            .iter()
            .filter(|node| {
                if let Some(external_addresses) = node.alternate_addresses.get("external") {
                    return external_addresses.ports.contains_key(key);
                }

                false
            })
            .map(|node| {
                let address = node.alternate_addresses.get("external").unwrap();
                let hostname = if address.hostname.is_some() {
                    address.hostname.as_ref().unwrap().clone()
                } else {
                    self.loaded_from.as_ref().unwrap().clone()
                };
                (hostname, *address.ports.get(key).unwrap())
            })
            .collect();

        for seed in &external {
            if seed.0 == self.loaded_from.as_ref().unwrap().clone() {
                return external;
            }
        }

        default
    }

    fn random_management_seed(&self, tls: bool) -> Option<(String, u32)> {
        self.random_seed(self.management_seeds(tls))
    }

    fn random_query_seed(&self, tls: bool) -> Option<(String, u32)> {
        self.random_seed(self.query_seeds(tls))
    }

    fn random_analytics_seed(&self, tls: bool) -> Option<(String, u32)> {
        self.random_seed(self.analytics_seeds(tls))
    }

    fn random_search_seed(&self, tls: bool) -> Option<(String, u32)> {
        self.random_seed(self.search_seeds(tls))
    }

    fn random_seed(&self, seeds: Vec<(String, u32)>) -> Option<(String, u32)> {
        let mut rng = rand::thread_rng();

        if seeds.is_empty() {
            return None;
        }

        let seed_idx = rng.gen_range(0..seeds.len());
        let seed = seeds.get(seed_idx);

        if let Some(s) = seed {
            return Some((s.0.clone(), s.1));
        }

        None
    }
}
