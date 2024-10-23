use crate::client::error::{ClientError, ConfigurationLoadFailedReason};
use crate::client::http_handler::{HTTPHandler, HttpResponse, HttpVerb};
use crate::client::kv_client::NodeExtConfig;
use crate::RustTlsConfig;
use bytes::Bytes;
use futures_core::Stream;
use log::{debug, trace};
use rand::Rng;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::json;
use std::fmt::{Debug, Display, Formatter};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;
use std::{collections::HashMap, ops::Sub};
use tokio::runtime::Runtime;
use tokio::time::Instant;

const CLUSTER_CONFIG_URI: &str = "/pools/default/nodeServices";

pub struct HTTPClient {
    seeds: Vec<String>,
    tls_enabled: bool,
    http_client: HTTPHandler,
}

impl HTTPClient {
    pub fn new(
        seeds: Vec<String>,
        username: String,
        password: String,
        tls_config: Option<RustTlsConfig>,
    ) -> Self {
        let tls_enabled = tls_config.is_some();
        Self {
            seeds,
            http_client: HTTPHandler::new(username, password, tls_config),
            tls_enabled,
        }
    }

    pub(crate) async fn get_config<T>(
        seeds: &Vec<String>,
        tls_enabled: bool,
        http_agent: &HTTPHandler,
        bucket: impl Into<Option<String>>,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<T, ClientError>
    where
        T: DeserializeOwned + Debug + Config,
    {
        let bucket = bucket.into();
        let path = match &bucket {
            Some(b) => format!("/pools/default/b/{}", b),
            None => CLUSTER_CONFIG_URI.to_string(),
        };
        let mut final_error_reason = None;
        let mut final_error_status = 0;
        for seed in seeds {
            let host_split: Vec<String> = seed.split(':').map(|v| v.to_owned()).collect();

            let host: String;
            let port: i32;
            if host_split.len() == 1 {
                host = seed.clone();
                port = if tls_enabled { 18091 } else { 8091 };
            } else {
                host = host_split[0].clone();
                port = match host_split[1].parse::<i32>() {
                    Ok(p) => p,
                    Err(e) => {
                        final_error_reason =
                            Some(format!("Failed to get port from seed {}: {}", &seed, e));
                        continue;
                    }
                }
            }

            let uri = format!("{}:{}{}", host, port, &path);

            debug!("Fetching config from {}", uri);

            let (content, status) = match http_agent.http_get(&uri, deadline, ctrl_c.clone()).await
            {
                Ok((content, status)) => (content, status),
                Err(e) => {
                    final_error_reason = Some(e.expanded_message());
                    continue;
                }
            };
            if status != 200 {
                if !content.is_empty() {
                    final_error_reason = Some(content);
                }
                final_error_status = status;
                continue;
            }
            let mut config: T = match serde_json::from_str(&content) {
                Ok(c) => c,
                Err(e) => {
                    final_error_reason =
                        Some(format!("Failed to deserialize cluster config: {}", e));
                    continue;
                }
            };
            config.set_loaded_from(host);

            trace!("Fetched config {:?}", &config);

            return Ok(config);
        }

        let reason = match final_error_status {
            401 => ConfigurationLoadFailedReason::Unauthorized,
            403 => ConfigurationLoadFailedReason::Forbidden,
            404 => ConfigurationLoadFailedReason::NotFound { bucket },
            _ => match final_error_reason {
                Some(reason) => {
                    if reason.contains("timed out") {
                        return Err(ClientError::ClusterNotContactable {
                            cluster: seeds.join(","),
                            reason: "timeout".to_string(),
                        });
                    } else if reason.contains("connect error") {
                        return Err(ClientError::ClusterNotContactable {
                            cluster: seeds.join(","),
                            reason: "connect error".to_string(),
                        });
                    }
                    ConfigurationLoadFailedReason::Unknown { reason }
                }
                None => ConfigurationLoadFailedReason::Unknown {
                    reason: format!(
                        "Failed to load config object for an unknown reason. Status code: {}",
                        final_error_status
                    ),
                },
            },
        };
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

        let mut state = "OK".to_string();
        if error.is_some() {
            state = "Error".to_string();
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
            let config: ClusterConfig = HTTPClient::get_config(
                &self.seeds,
                self.tls_enabled,
                &self.http_client,
                None,
                deadline,
                ctrl_c.clone(),
            )
            .await?;

            let mut results: Vec<PingResponse> = Vec::new();
            for seed in config.search_seeds(self.tls_enabled) {
                let uri = format!("{}:{}/api/ping", seed.hostname(), seed.port());
                let address = format!("{}:{}", seed.hostname(), seed.port());
                results.push(
                    self.ping_endpoint(uri, address, ServiceType::Search, deadline, ctrl_c.clone())
                        .await?,
                );
            }
            for seed in config.query_seeds(self.tls_enabled) {
                let uri = format!("{}:{}/admin/ping", seed.hostname(), seed.port());
                let address = format!("{}:{}", seed.hostname(), seed.port());
                results.push(
                    self.ping_endpoint(uri, address, ServiceType::Query, deadline, ctrl_c.clone())
                        .await?,
                );
            }
            for seed in config.analytics_seeds(self.tls_enabled) {
                let uri = format!("{}:{}/admin/ping", seed.hostname(), seed.port());
                let address = format!("{}:{}", seed.hostname(), seed.port());
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
            for seed in config.view_seeds(self.tls_enabled) {
                let uri = format!("{}:{}/", seed.hostname(), seed.port());
                let address = format!("{}:{}", seed.hostname(), seed.port());
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
            let config: ClusterConfig = HTTPClient::get_config(
                &self.seeds,
                self.tls_enabled,
                &self.http_client,
                None,
                deadline,
                ctrl_c.clone(),
            )
            .await?;

            let path = request.path();
            if let Some(seed) = config.random_management_seed(self.tls_enabled) {
                let uri = format!("{}:{}{}", seed.hostname(), seed.port(), &path);
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
                return Ok(HttpResponse::new(content, status, seed));
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
            let config: ClusterConfig = HTTPClient::get_config(
                &self.seeds,
                self.tls_enabled,
                &self.http_client,
                None,
                deadline,
                ctrl_c.clone(),
            )
            .await?;

            let seed = if let Some(e) = request.endpoint() {
                if !config.has_query_seed(&e, self.tls_enabled) {
                    return Err(ClientError::RequestFailed {
                        reason: Some(format!("Endpoint {} not known", e)),
                        key: None,
                    });
                }
                e
            } else if let Some(s) = config.random_query_seed(self.tls_enabled) {
                s
            } else {
                return Err(ClientError::RequestFailed {
                    reason: Some("No nodes found for service".to_string()),
                    key: None,
                });
            };

            let path = request.path();
            let uri = format!("{}:{}{}", seed.hostname(), seed.port(), &path);
            let (content, status) = match request.verb() {
                HttpVerb::Get => self.http_client.http_get(&uri, deadline, ctrl_c).await?,
                HttpVerb::Post => {
                    self.http_client
                        .http_post(&uri, request.payload(), request.headers(), deadline, ctrl_c)
                        .await?
                }
                _ => {
                    return Err(ClientError::RequestFailed {
                        reason: Some("Method not allowed for queries".to_string()),
                        key: None,
                    });
                }
            };

            Ok(HttpResponse::new(content, status, seed))
        })
    }

    pub async fn analytics_query_stream_request(
        &self,
        request: AnalyticsQueryRequest,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<
        (
            impl Stream<Item = Result<Bytes, reqwest::Error>> + Sized,
            u16,
        ),
        ClientError,
    > {
        let config: ClusterConfig = HTTPClient::get_config(
            &self.seeds,
            self.tls_enabled,
            &self.http_client,
            None,
            deadline,
            ctrl_c.clone(),
        )
        .await?;

        let path = request.path();
        if let Some(seed) = config.random_analytics_seed(self.tls_enabled) {
            let uri = format!("{}:{}{}", seed.hostname(), seed.port(), &path);
            let (stream, status) = match request.verb() {
                // HttpVerb::Get => self.http_client.http_get(&uri, deadline, ctrl_c).await?,
                HttpVerb::Post => {
                    self.http_client
                        .http_post_stream(
                            &uri,
                            request.payload(),
                            request.headers(),
                            deadline,
                            ctrl_c,
                        )
                        .await?
                }
                _ => {
                    return Err(ClientError::RequestFailed {
                        reason: Some("Method not allowed for analytics queries".to_string()),
                        key: None,
                    });
                }
            };

            return Ok((stream, status));
        }

        Err(ClientError::RequestFailed {
            reason: Some("No nodes found for service".to_string()),
            key: None,
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
            let config: ClusterConfig = HTTPClient::get_config(
                &self.seeds,
                self.tls_enabled,
                &self.http_client,
                None,
                deadline,
                ctrl_c.clone(),
            )
            .await?;

            let path = request.path();
            if let Some(seed) = config.random_analytics_seed(self.tls_enabled) {
                let uri = format!("{}:{}{}", seed.hostname(), seed.port(), &path);
                let (content, status) = match request.verb() {
                    HttpVerb::Get => self.http_client.http_get(&uri, deadline, ctrl_c).await?,
                    HttpVerb::Post => {
                        self.http_client
                            .http_post(&uri, request.payload(), request.headers(), deadline, ctrl_c)
                            .await?
                    }
                    _ => {
                        return Err(ClientError::RequestFailed {
                            reason: Some("Method not allowed for analytics queries".to_string()),
                            key: None,
                        });
                    }
                };

                return Ok(HttpResponse::new(content, status, seed));
            }

            Err(ClientError::RequestFailed {
                reason: Some("No nodes found for service".to_string()),
                key: None,
            })
        })
    }

    pub fn search_query_request(
        &self,
        request: impl SearchQueryRequest,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<HttpResponse, ClientError> {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let config: ClusterConfig = HTTPClient::get_config(
                &self.seeds,
                self.tls_enabled,
                &self.http_client,
                None,
                deadline,
                ctrl_c.clone(),
            )
            .await?;

            let path = request.path();
            if let Some(seed) = config.random_search_seed(self.tls_enabled) {
                let uri = format!("{}:{}{}", seed.hostname(), seed.port(), &path);
                let (content, status) = match request.verb() {
                    HttpVerb::Post => {
                        self.http_client
                            .http_post(&uri, request.payload(), request.headers(), deadline, ctrl_c)
                            .await?
                    }
                    _ => {
                        return Err(ClientError::RequestFailed {
                            reason: Some("Method not allowed for analytics queries".to_string()),
                            key: None,
                        });
                    }
                };

                return Ok(HttpResponse::new(content, status, seed));
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
    VectorCreateIndex {
        bucket: String,
        scope: String,
        name: String,
        payload: String,
    },
}

impl ManagementRequest {
    pub fn path(&self) -> String {
        match self {
            Self::GetBuckets => "/pools/default/buckets".to_string(),
            Self::GetBucket { name } => format!("/pools/default/buckets/{}", name),
            Self::IndexStatus => "/indexStatus".to_string(),
            Self::SettingsAutoFailover => "/settings/autoFailover".to_string(),
            Self::BucketStats { name } => format!("/pools/default/buckets/{}/stats", name),
            Self::CreateBucket { .. } => "/pools/default/buckets".to_string(),
            Self::DropBucket { name } => format!("/pools/default/buckets/{}", name),
            Self::DropUser { username } => format!("/settings/rbac/users/local/{}", username),
            Self::FlushBucket { name } => {
                format!("/pools/default/buckets/{}/controller/doFlush", name)
            }
            Self::LoadSampleBucket { .. } => "/sampleBuckets/install".to_string(),
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
            Self::GetNodes => "/pools/default".to_string(),
            Self::GetUsers => "/settings/rbac/users/local".to_string(),
            Self::GetUser { username } => format!("/settings/rbac/users/local/{}", username),
            Self::GetRoles { permission } => match permission {
                Some(p) => format!("/settings/rbac/roles?permission={}", p),
                None => "/settings/rbac/roles".to_string(),
            },
            Self::UpsertUser { username, .. } => format!("/settings/rbac/users/local/{}", username),
            Self::CreateScope { bucket, .. } => {
                format!("/pools/default/buckets/{}/scopes", bucket)
            }
            Self::DropScope { bucket, name } => {
                format!("/pools/default/buckets/{}/scopes/{}", bucket, name)
            }
            Self::GetScopes { bucket } => format!("/pools/default/buckets/{}/scopes", bucket),
            Self::VectorCreateIndex {
                bucket,
                scope,
                name,
                ..
            } => format!(
                "/_p/fts/api/bucket/{}/scope/{}/index/{}",
                bucket, scope, name
            ),
        }
    }

    pub fn verb(&self) -> HttpVerb {
        match self {
            Self::GetBuckets => HttpVerb::Get,
            Self::GetBucket { .. } => HttpVerb::Get,
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
            Self::VectorCreateIndex { .. } => HttpVerb::Put,
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
            Self::VectorCreateIndex { payload, .. } => Some(payload.as_bytes().into()),
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
            Self::CreateScope { .. } => {
                let mut h = HashMap::new();
                h.insert("Content-Type", "application/x-www-form-urlencoded");
                h
            }
            _ => HashMap::new(),
        }
    }
}

pub struct QueryTransactionRequest {
    tx_timeout: Option<Duration>,
    tx_id: Option<String>,
    endpoint: Option<Endpoint>,
}

impl QueryTransactionRequest {
    pub fn new(
        tx_timeout: impl Into<Option<Duration>>,
        tx_id: impl Into<Option<String>>,
        endpoint: impl Into<Option<Endpoint>>,
    ) -> Self {
        Self {
            tx_timeout: tx_timeout.into(),
            tx_id: tx_id.into(),
            endpoint: endpoint.into(),
        }
    }
}

pub enum QueryRequest {
    Execute {
        statement: String,
        parameters: Option<serde_json::Value>,
        scope: Option<(String, String)>,
        timeout: String,
        transaction: Option<QueryTransactionRequest>,
    },
}

impl QueryRequest {
    pub fn path(&self) -> String {
        match self {
            Self::Execute { .. } => "/query".to_string(),
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
                transaction,
                parameters,
            } => {
                let mut json = HashMap::new();
                if let Some(scope) = scope {
                    let ctx = format!("`default`:`{}`.`{}`", scope.0, scope.1);
                    json.insert("query_context".to_string(), serde_json::Value::String(ctx));
                }

                json.insert(
                    "statement".to_string(),
                    serde_json::Value::String(statement.to_string()),
                );
                json.insert(
                    "timeout".to_string(),
                    serde_json::Value::String(timeout.to_string()),
                );
                if let Some(txn) = transaction {
                    if let Some(t) = txn.tx_timeout {
                        json.insert(
                            "txtimeout".to_string(),
                            serde_json::Value::String(format!("{}ms", t.as_millis())),
                        );
                    }
                    if let Some(id) = txn.tx_id.clone() {
                        json.insert("txid".to_string(), serde_json::Value::String(id));
                    }
                }

                if let Some(params) = parameters {
                    match params {
                        serde_json::Value::Array(_) => {
                            json.insert("args".to_string(), params.clone());
                        }
                        serde_json::Value::Object(map) => {
                            for (k, v) in map.iter() {
                                let key = if k.starts_with('$') {
                                    k.clone()
                                } else {
                                    format!("${}", *k)
                                };
                                json.insert(key, v.clone());
                            }
                        }
                        _ => {}
                    }
                }

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

    pub fn endpoint(&self) -> Option<Endpoint> {
        match self {
            Self::Execute { transaction, .. } => {
                if let Some(txn) = transaction {
                    if let Some(endpoint) = txn.endpoint.clone() {
                        return Some(endpoint);
                    }
                }
                None
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
            Self::Execute { .. } => "/query/service".to_string(),
            Self::PendingMutations => "/analytics/node/agg/stats/remaining".to_string(),
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

pub trait SearchQueryRequest {
    fn path(&self) -> String;
    fn verb(&self) -> HttpVerb;
    fn payload(&self) -> Option<Vec<u8>>;
    fn headers(&self) -> HashMap<&str, &str>;
}

pub enum TextSearchQueryRequest {
    Execute {
        index: String,
        query: String,
        timeout: u128,
    },
}

impl SearchQueryRequest for TextSearchQueryRequest {
    fn path(&self) -> String {
        match self {
            Self::Execute { index, .. } => format!("/api/index/{}/query", index),
        }
    }

    fn verb(&self) -> HttpVerb {
        match self {
            Self::Execute { .. } => HttpVerb::Post,
        }
    }

    fn payload(&self) -> Option<Vec<u8>> {
        match self {
            Self::Execute { query, timeout, .. } => {
                let json = json!({ "query": { "query": query }, "ctl": { "timeout": timeout }});
                Some(serde_json::to_vec(&json).unwrap())
            }
        }
    }

    fn headers(&self) -> HashMap<&str, &str> {
        match self {
            Self::Execute { .. } => {
                let mut h = HashMap::new();
                h.insert("Content-Type", "application/json");
                h
            }
        }
    }
}

pub enum VectorSearchQueryRequest {
    Execute {
        index: String,
        query: serde_json::Value,
        vector: Vec<f32>,
        field: String,
        neighbors: i64,
        timeout: u128,
    },
}

impl SearchQueryRequest for VectorSearchQueryRequest {
    fn path(&self) -> String {
        match self {
            Self::Execute { index, .. } => format!("/api/index/{}/query", index),
        }
    }

    fn verb(&self) -> HttpVerb {
        match self {
            Self::Execute { .. } => HttpVerb::Post,
        }
    }

    fn payload(&self) -> Option<Vec<u8>> {
        match self {
            Self::Execute {
                query,
                timeout,
                vector,
                field,
                neighbors,
                ..
            } => {
                let json = json!({ "query":  query, "knn" :[{"field": field, "k": neighbors, "vector":vector}], "ctl": { "timeout": timeout }});
                Some(serde_json::to_vec(&json).unwrap())
            }
        }
    }

    fn headers(&self) -> HashMap<&str, &str> {
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
            ServiceType::KeyValue => "KeyValue".to_string(),
            ServiceType::Query => "Query".to_string(),
            ServiceType::Search => "Search".to_string(),
            ServiceType::Analytics => "Analytics".to_string(),
            ServiceType::Views => "Views".to_string(),
        }
    }
}

pub(crate) trait Config {
    fn set_loaded_from(&mut self, loaded_from: String);
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct Endpoint {
    hostname: String,
    port: u32,
}

impl Display for Endpoint {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.hostname, self.port)
    }
}

impl Endpoint {
    pub fn new(hostname: String, port: u32) -> Self {
        Self { hostname, port }
    }

    pub fn hostname(&self) -> &str {
        &self.hostname
    }

    pub fn port(&self) -> u32 {
        self.port
    }
}

#[derive(Deserialize, Debug)]
pub(crate) struct ClusterConfig {
    // rev: u64,
    #[serde(alias = "nodesExt")]
    nodes_ext: Vec<NodeExtConfig>,
    loaded_from: Option<String>,
}

impl Config for ClusterConfig {
    fn set_loaded_from(&mut self, loaded_from: String) {
        self.loaded_from = Some(loaded_from);
    }
}

impl ClusterConfig {
    pub fn management_seeds(&self, tls: bool) -> Vec<Endpoint> {
        let key = if tls { "mgmtSSL" } else { "mgmt" };

        self.seeds(key)
    }

    pub fn query_seeds(&self, tls: bool) -> Vec<Endpoint> {
        let key = if tls { "n1qlSSL" } else { "n1ql" };

        self.seeds(key)
    }

    pub fn analytics_seeds(&self, tls: bool) -> Vec<Endpoint> {
        let key = if tls { "cbasSSL" } else { "cbas" };

        self.seeds(key)
    }

    pub fn search_seeds(&self, tls: bool) -> Vec<Endpoint> {
        let key = if tls { "ftsSSL" } else { "fts" };

        self.seeds(key)
    }

    pub fn view_seeds(&self, tls: bool) -> Vec<Endpoint> {
        let key = if tls { "capiSSL" } else { "capi" };

        self.seeds(key)
    }

    fn seeds(&self, key: &str) -> Vec<Endpoint> {
        let default: Vec<Endpoint> = self
            .nodes_ext
            .iter()
            .filter(|node| node.services.contains_key(key))
            .map(|node| {
                let hostname = if node.hostname.is_some() {
                    node.hostname.as_ref().unwrap().clone()
                } else {
                    self.loaded_from.as_ref().unwrap().clone()
                };
                Endpoint::new(hostname, *node.services.get(key).unwrap())
            })
            .collect();

        for seed in &default {
            if seed.hostname() == self.loaded_from.as_ref().unwrap().clone() {
                return default;
            }
        }

        let external: Vec<Endpoint> = self
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
                Endpoint::new(hostname, *address.ports.get(key).unwrap())
            })
            .collect();

        for seed in &external {
            if seed.hostname() == self.loaded_from.as_ref().unwrap().clone() {
                return external;
            }
        }

        default
    }

    pub fn has_query_seed(&self, endpoint: &Endpoint, tls: bool) -> bool {
        let seeds = self.query_seeds(tls);
        seeds.contains(endpoint)
    }

    fn random_management_seed(&self, tls: bool) -> Option<Endpoint> {
        self.random_seed(self.management_seeds(tls))
    }

    fn random_query_seed(&self, tls: bool) -> Option<Endpoint> {
        self.random_seed(self.query_seeds(tls))
    }

    pub fn random_analytics_seed(&self, tls: bool) -> Option<Endpoint> {
        self.random_seed(self.analytics_seeds(tls))
    }

    fn random_search_seed(&self, tls: bool) -> Option<Endpoint> {
        self.random_seed(self.search_seeds(tls))
    }

    fn random_seed(&self, seeds: Vec<Endpoint>) -> Option<Endpoint> {
        let mut rng = rand::thread_rng();

        if seeds.is_empty() {
            return None;
        }

        let seed_idx = rng.gen_range(0..seeds.len());
        let seed = seeds.get(seed_idx);

        if let Some(s) = seed {
            return Some(s.clone());
        }

        None
    }
}
