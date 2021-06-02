mod codec;
mod kv;
mod protocol;

use std::{collections::HashMap, ops::Sub};

use crate::cli::CtrlcFuture;
use crate::client::kv::KvEndpoint;
use crate::client::ClientError::CollectionNotFound;
use crate::config::ClusterTlsConfig;
use crc::crc32;
use hmac::{Hmac, Mac, NewMac};
use isahc::{
    auth::{Authentication, Credentials},
    config::CaCertificate,
    ResponseFuture,
};
use isahc::{config::SslOption, prelude::*};
use nu_errors::ShellError;
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::Sha256;
use std::fmt;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::runtime::Runtime;
use tokio::time::sleep;
use tokio::{select, time::Instant};

const CLOUD_URL: &str = "https://cloudapi.cloud.couchbase.com";

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Clone, Serialize, Deserialize, Hash)]
pub enum ClientError {
    ConfigurationLoadFailed { reason: Option<String> },
    CollectionManifestLoadFailed { reason: Option<String> },
    CollectionNotFound,
    ScopeNotFound,
    KeyNotFound,
    KeyAlreadyExists,
    AccessError,
    AuthError,
    Timeout,
    Cancelled,
    ClusterNotFound { name: String },
    RequestFailed { reason: Option<String> },
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let message = match self {
            Self::ConfigurationLoadFailed { reason } => match reason.as_ref() {
                Some(re) => format!("failed to load config from cluster: {}", re),
                None => "failed to load config from cluster".into(),
            },
            Self::CollectionManifestLoadFailed { reason } => match reason.as_ref() {
                Some(re) => format!("failed to load collection manifest from cluster: {}", re),
                None => "failed to load collection manifest from cluster".into(),
            },
            Self::CollectionNotFound => "collection not found".into(),
            Self::ScopeNotFound => "scope not found".into(),
            Self::KeyNotFound => "key not found".into(),
            Self::KeyAlreadyExists => "key already exists".into(),
            Self::AccessError => "access error".into(),
            Self::AuthError => "authentication error".into(),
            Self::Timeout => "timeout".into(),
            Self::Cancelled => "request cancelled".into(),
            Self::ClusterNotFound { name } => format!("cluster not found: {}", name),
            Self::RequestFailed { reason } => match reason.as_ref() {
                Some(re) => format!("request failed: {}", re),
                None => "request failed".into(),
            },
        };
        write!(f, "{}", message)
    }
}

impl From<ClientError> for ShellError {
    fn from(ce: ClientError) -> Self {
        // todo: this can definitely be improved with more detail and reporting specifics
        ShellError::untagged_runtime_error(ce.to_string())
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

impl From<serde_json::Error> for ClientError {
    fn from(e: serde_json::Error) -> Self {
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

#[derive(Debug, Deserialize)]
struct LimitedClusterSummary {
    id: String,
    name: String,
}

pub struct CloudClient {
    secret_key: String,
    access_key: String,
}

impl CloudClient {
    pub fn new(secret_key: String, access_key: String) -> Self {
        Self {
            secret_key,
            access_key,
        }
    }

    fn http_do(
        &self,
        verb: HttpVerb,
        path: &str,
        payload: Option<Vec<u8>>,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<(String, u16), ClientError> {
        let now = Instant::now();
        if now >= deadline {
            return Err(ClientError::Timeout);
        }
        let timeout = deadline.sub(now);
        let ctrl_c_fut = CtrlcFuture::new(ctrl_c);

        let uri = format!("{}{}", CLOUD_URL, path);

        let mut res_builder = match verb {
            HttpVerb::Get => isahc::Request::get(uri),
            HttpVerb::Delete => isahc::Request::delete(uri),
            HttpVerb::Put => isahc::Request::put(uri),
            HttpVerb::Post => isahc::Request::post(uri),
        };

        let now_millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();

        let bearer_payload = format!(
            "{}\n{}\n{}",
            res_builder.method_ref().unwrap(),
            path,
            now_millis.to_string()
        );

        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(self.secret_key.clone().as_bytes()).unwrap();
        mac.update(bearer_payload.as_bytes());
        let mac_result = mac.finalize();

        let bearer = format!(
            "Bearer {}:{}",
            self.access_key.clone(),
            base64::encode(mac_result.into_bytes())
        );

        res_builder = res_builder
            .timeout(timeout)
            .header("content-type", "application/json")
            .header("Authorization", bearer)
            .header("Couchbase-Timestamp", now_millis.to_string());

        let res_fut: ResponseFuture;
        if let Some(p) = payload {
            res_fut = res_builder.body(p)?.send_async();
        } else {
            res_fut = res_builder.body(())?.send_async();
        }

        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            select! {
                result = res_fut => {
                    let mut response = result.map_err(ClientError::from)?;
                    let content = response.text().await?;
                    let status = response.status().into();
                    Ok((content, status))
                },
                () = ctrl_c_fut => Err(ClientError::Cancelled),
            }
        })
    }

    fn http_get(
        &self,
        path: &str,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<(String, u16), ClientError> {
        self.http_do(HttpVerb::Get, path, None, deadline, ctrl_c)
    }

    fn http_delete(
        &self,
        path: &str,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<(String, u16), ClientError> {
        self.http_do(HttpVerb::Delete, path, None, deadline, ctrl_c)
    }

    fn http_post(
        &self,
        path: &str,
        payload: Option<Vec<u8>>,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<(String, u16), ClientError> {
        self.http_do(HttpVerb::Post, path, payload, deadline, ctrl_c)
    }

    fn http_put(
        &self,
        path: &str,
        payload: Option<Vec<u8>>,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<(String, u16), ClientError> {
        self.http_do(HttpVerb::Put, path, payload, deadline, ctrl_c)
    }

    pub fn find_cluster(
        &self,
        cluster_name: String,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<String, ClientError> {
        let request = CloudRequest::GetClusters {};
        let (content, status) = self.http_get(request.path().as_str(), deadline, ctrl_c)?;

        if status != 200 {
            return Err(ClientError::RequestFailed {
                reason: Some(content),
            });
        }

        let data: Value = serde_json::from_str(content.as_str())?;
        let v = data.get("data").unwrap().to_string();
        let clusters: Vec<LimitedClusterSummary> = serde_json::from_str(v.as_str())?;

        for c in clusters {
            if c.name == cluster_name {
                return Ok(c.id);
            }
        }

        Err(ClientError::ClusterNotFound { name: cluster_name })
    }

    pub fn cloud_request(
        &self,
        request: CloudRequest,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<HttpResponse, ClientError> {
        let (content, status) = match request.verb() {
            HttpVerb::Get => self.http_get(request.path().as_str(), deadline, ctrl_c)?,
            HttpVerb::Post => {
                self.http_post(request.path().as_str(), request.payload(), deadline, ctrl_c)?
            }
            HttpVerb::Delete => self.http_delete(request.path().as_str(), deadline, ctrl_c)?,
            HttpVerb::Put => {
                self.http_put(request.path().as_str(), request.payload(), deadline, ctrl_c)?
            }
        };
        Ok(HttpResponse { content, status })
    }
}

pub struct Client {
    seeds: Vec<String>,
    username: String,
    password: String,
    tls_config: ClusterTlsConfig,
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
            tls_config,
        }
    }

    fn http_prefix(&self) -> &'static str {
        match self.tls_config.enabled() {
            true => "https",
            false => "http",
        }
    }

    fn status_to_reason(&self, status: u16) -> Option<String> {
        match status {
            400 => Some("bad request".into()),
            401 => Some("unauthorized".into()),
            403 => Some("forbidden".into()),
            404 => Some("not found".into()),
            _ => None,
        }
    }

    fn get_config(
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
                    })?;
            }

            let uri = format!("{}://{}:{}{}", self.http_prefix(), host, port, &path);
            let (content, status) = self.http_get(&uri, deadline, ctrl_c.clone())?;
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
            reason = self.status_to_reason(final_error_status);
        }
        Err(ClientError::ConfigurationLoadFailed { reason })
    }

    fn get_bucket_config(
        &self,
        bucket: String,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<BucketConfig, ClientError> {
        let path = format!("/pools/default/b/{}", bucket);
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
                    })?;
            }

            let uri = format!("{}://{}:{}{}", self.http_prefix(), host, port, &path);
            let (content, status) = self.http_get(&uri, deadline, ctrl_c.clone())?;
            if status != 200 {
                if !content.is_empty() {
                    final_error_content = Some(content);
                }
                final_error_status = status;
                continue;
            }
            let mut config: BucketConfig = serde_json::from_str(&content).unwrap();
            config.set_loaded_from(host);
            return Ok(config);
        }
        let mut reason = final_error_content;
        if reason.is_none() {
            reason = self.status_to_reason(final_error_status);
        }
        Err(ClientError::ConfigurationLoadFailed { reason })
    }

    fn get_collection_manifest(
        &self,
        bucket: String,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<CollectionManifest, ClientError> {
        let path = format!("/pools/default/buckets/{}/scopes/", bucket);
        let port = if self.tls_config.enabled() {
            18091
        } else {
            8091
        };
        let mut final_error_content = None;
        let mut final_error_status = 0;
        for seed in &self.seeds {
            let uri = format!("{}://{}:{}{}", self.http_prefix(), seed, port, &path);
            let (content, status) = self.http_get(&uri, deadline, ctrl_c.clone())?;
            if status != 200 {
                if !content.is_empty() {
                    final_error_content = Some(content);
                }
                final_error_status = status;
                continue;
            }
            let manifest: CollectionManifest = serde_json::from_str(&content).unwrap();
            return Ok(manifest);
        }
        let mut reason = final_error_content;
        if reason.is_none() {
            reason = self.status_to_reason(final_error_status);
        }
        Err(ClientError::CollectionManifestLoadFailed { reason })
    }

    fn http_do(
        &self,
        mut res_builder: http::request::Builder,
        payload: Option<Vec<u8>>,
        headers: HashMap<&str, &str>,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<(String, u16), ClientError> {
        let now = Instant::now();
        if now >= deadline {
            return Err(ClientError::Timeout);
        }
        let timeout = deadline.sub(now);
        let ctrl_c_fut = CtrlcFuture::new(ctrl_c);

        res_builder = res_builder
            .authentication(Authentication::basic())
            .credentials(Credentials::new(&self.username, &self.password))
            .timeout(timeout);

        if self.tls_config.enabled() {
            if let Some(cert) = self.tls_config.cert_path() {
                res_builder = res_builder.ssl_ca_certificate(CaCertificate::file(cert));
            }
            res_builder = res_builder.ssl_options(self.http_ssl_opts());
        }

        for (key, value) in headers {
            res_builder = res_builder.header(key, value);
        }

        let res_fut: ResponseFuture;
        if let Some(p) = payload {
            res_fut = res_builder.body(p)?.send_async();
        } else {
            res_fut = res_builder.body(())?.send_async();
        }

        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            select! {
                result = res_fut => {
                    let mut response = result.map_err(ClientError::from)?;
                    let content = response.text().await?;
                    let status = response.status().into();
                    Ok((content, status))
                },
                () = ctrl_c_fut => Err(ClientError::Cancelled),
            }
        })
    }

    fn http_get(
        &self,
        uri: &str,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<(String, u16), ClientError> {
        let res_builder = isahc::Request::get(uri);
        self.http_do(res_builder, None, HashMap::new(), deadline, ctrl_c)
    }

    fn http_delete(
        &self,
        uri: &str,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<(String, u16), ClientError> {
        let res_builder = isahc::Request::delete(uri);
        self.http_do(res_builder, None, HashMap::new(), deadline, ctrl_c)
    }

    fn http_ssl_opts(&self) -> SslOption {
        let mut ssl_opts = SslOption::NONE;
        if !self.tls_config.validate_hostnames() {
            ssl_opts |= SslOption::DANGER_ACCEPT_INVALID_HOSTS;
        }
        if self.tls_config.accept_all_certs() {
            ssl_opts |= SslOption::DANGER_ACCEPT_INVALID_CERTS;
        }
        ssl_opts
    }

    fn http_post(
        &self,
        uri: &str,
        payload: Option<Vec<u8>>,
        headers: HashMap<&str, &str>,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<(String, u16), ClientError> {
        let res_builder = isahc::Request::post(uri);
        self.http_do(res_builder, payload, headers, deadline, ctrl_c)
    }

    fn http_put(
        &self,
        uri: &str,
        payload: Option<Vec<u8>>,
        headers: HashMap<&str, &str>,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<(String, u16), ClientError> {
        let res_builder = isahc::Request::put(uri);
        self.http_do(res_builder, payload, headers, deadline, ctrl_c)
    }

    fn ping_endpoint(
        &self,
        uri: String,
        address: String,
        service: ServiceType,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<PingResponse, ClientError> {
        let start = Instant::now();
        let result = self.http_get(&uri, deadline, ctrl_c);
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
    pub fn ping_all_http_request(
        &self,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<Vec<PingResponse>, ClientError> {
        let config = self.get_config(deadline, ctrl_c.clone())?;

        let mut results: Vec<PingResponse> = Vec::new();
        for seed in config.search_seeds(self.tls_config.enabled()) {
            let uri = format!("{}://{}:{}/api/ping", self.http_prefix(), seed.0, seed.1);
            let address = format!("{}:{}", seed.0, seed.1);
            results.push(self.ping_endpoint(
                uri,
                address,
                ServiceType::Search,
                deadline,
                ctrl_c.clone(),
            )?);
        }
        for seed in config.query_seeds(self.tls_config.enabled()) {
            let uri = format!("{}://{}:{}/admin/ping", self.http_prefix(), seed.0, seed.1);
            let address = format!("{}:{}", seed.0, seed.1);
            results.push(self.ping_endpoint(
                uri,
                address,
                ServiceType::Query,
                deadline,
                ctrl_c.clone(),
            )?);
        }
        for seed in config.analytics_seeds(self.tls_config.enabled()) {
            let uri = format!("{}://{}:{}/admin/ping", self.http_prefix(), seed.0, seed.1);
            let address = format!("{}:{}", seed.0, seed.1);
            results.push(self.ping_endpoint(
                uri,
                address,
                ServiceType::Analytics,
                deadline,
                ctrl_c.clone(),
            )?);
        }
        for seed in config.view_seeds(self.tls_config.enabled()) {
            let uri = format!("{}://{}:{}/", self.http_prefix(), seed.0, seed.1);
            let address = format!("{}:{}", seed.0, seed.1);
            results.push(self.ping_endpoint(
                uri,
                address,
                ServiceType::Views,
                deadline,
                ctrl_c.clone(),
            )?);
        }

        Ok(results)
    }

    pub fn management_request(
        &self,
        request: ManagementRequest,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<HttpResponse, ClientError> {
        let config = self.get_config(deadline, ctrl_c.clone())?;

        let path = request.path();
        if let Some(seed) = config.random_management_seed(self.tls_config.enabled()) {
            let uri = format!("{}://{}:{}{}", self.http_prefix(), seed.0, seed.1, &path);
            let (content, status) = match request.verb() {
                HttpVerb::Get => self.http_get(&uri, deadline, ctrl_c)?,
                HttpVerb::Post => {
                    self.http_post(&uri, request.payload(), request.headers(), deadline, ctrl_c)?
                }
                HttpVerb::Delete => self.http_delete(&uri, deadline, ctrl_c)?,
                HttpVerb::Put => {
                    self.http_put(&uri, request.payload(), request.headers(), deadline, ctrl_c)?
                }
            };
            return Ok(HttpResponse { content, status });
        }

        Err(ClientError::RequestFailed {
            reason: Some("No nodes found for service".to_string()),
        })
    }

    pub fn query_request(
        &self,
        request: QueryRequest,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<HttpResponse, ClientError> {
        let config = self.get_config(deadline, ctrl_c.clone())?;

        let path = request.path();
        if let Some(seed) = config.random_query_seed(self.tls_config.enabled()) {
            let uri = format!("{}://{}:{}{}", self.http_prefix(), seed.0, seed.1, &path);
            let (content, status) = match request.verb() {
                HttpVerb::Get => self.http_get(&uri, deadline, ctrl_c)?,
                HttpVerb::Post => {
                    self.http_post(&uri, request.payload(), request.headers(), deadline, ctrl_c)?
                }
                _ => {
                    return Err(ClientError::RequestFailed {
                        reason: Some("Method not allowed for queries".into()),
                    });
                }
            };

            return Ok(HttpResponse { content, status });
        }

        Err(ClientError::RequestFailed {
            reason: Some("No nodes found for service".to_string()),
        })
    }

    pub fn analytics_query_request(
        &self,
        request: AnalyticsQueryRequest,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<HttpResponse, ClientError> {
        let config = self.get_config(deadline, ctrl_c.clone())?;

        let path = request.path();
        if let Some(seed) = config.random_analytics_seed(self.tls_config.enabled()) {
            let uri = format!("{}://{}:{}{}", self.http_prefix(), seed.0, seed.1, &path);
            let (content, status) = match request.verb() {
                HttpVerb::Get => self.http_get(&uri, deadline, ctrl_c)?,
                HttpVerb::Post => {
                    self.http_post(&uri, request.payload(), request.headers(), deadline, ctrl_c)?
                }
                _ => {
                    return Err(ClientError::RequestFailed {
                        reason: Some("Method not allowed for analytics queries".into()),
                    });
                }
            };

            return Ok(HttpResponse { content, status });
        }

        Err(ClientError::RequestFailed {
            reason: Some("No nodes found for service".to_string()),
        })
    }

    pub fn search_query_request(
        &self,
        request: SearchQueryRequest,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<HttpResponse, ClientError> {
        let config = self.get_config(deadline, ctrl_c.clone())?;

        let path = request.path();
        if let Some(seed) = config.random_search_seed(self.tls_config.enabled()) {
            let uri = format!("{}://{}:{}{}", self.http_prefix(), seed.0, seed.1, &path);
            let (content, status) = match request.verb() {
                HttpVerb::Post => {
                    self.http_post(&uri, request.payload(), request.headers(), deadline, ctrl_c)?
                }
                _ => {
                    return Err(ClientError::RequestFailed {
                        reason: Some("Method not allowed for analytics queries".into()),
                    });
                }
            };

            return Ok(HttpResponse { content, status });
        }

        Err(ClientError::RequestFailed {
            reason: Some("No nodes found for service".to_string()),
        })
    }

    pub fn key_value_client(
        &self,
        username: String,
        password: String,
        bucket: String,
        scope: String,
        collection: String,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<KvClient, ClientError> {
        let config = self.get_bucket_config(bucket.clone(), deadline, ctrl_c.clone())?;
        let mut pair: Option<CollectionDetails> = None;
        if (!scope.is_empty() && scope != "_default")
            || (!collection.is_empty() && collection != "_default")
        {
            // If we've been specifically asked to use a scope or collection and fetching the manifest
            // fails then we need to report that.
            let manifest = self.get_collection_manifest(bucket.clone(), deadline, ctrl_c)?;
            pair = Some(CollectionDetails {
                scope,
                collection,
                manifest,
            })
        };

        Ok(KvClient {
            username,
            password,
            collection: pair,
            config,
            endpoints: HashMap::new(),
            tls_config: self.tls_config.clone(),
            bucket,
        })
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

pub enum CloudRequest {
    CreateAllowListEntry {
        cluster_id: String,
        payload: String,
    },
    CreateBucket {
        cluster_id: String,
        payload: String,
    },
    CreateCluster {
        payload: String,
    },
    CreateProject {
        payload: String,
    },
    CreateUser {
        cluster_id: String,
        payload: String,
    },
    DeleteAllowListEntry {
        cluster_id: String,
        payload: String,
    },
    DeleteBucket {
        cluster_id: String,
        payload: String,
    },
    DeleteCluster {
        cluster_id: String,
    },
    DeleteProject {
        project_id: String,
    },
    DeleteUser {
        cluster_id: String,
        username: String,
    },
    GetAPIStatus,
    GetAllowList {
        cluster_id: String,
    },
    GetBuckets {
        cluster_id: String,
    },
    GetCertificate {
        cluster_id: String,
    },
    GetCloud {
        cloud_id: String,
    },
    GetClouds,
    GetCluster {
        cluster_id: String,
    },
    GetClusterHealth {
        cluster_id: String,
    },
    GetClusters,
    GetClusterStatus {
        cluster_id: String,
    },
    GetProject {
        project_id: String,
    },
    GetProjects,
    GetUsers {
        cluster_id: String,
    },
    GetOrgUsers,
    UpdateAllowList {
        cluster_id: String,
        payload: String,
    },
    UpdateBucket {
        cluster_id: String,
        payload: String,
    },
    UpdateUser {
        cluster_id: String,
        payload: String,
    },
}

impl CloudRequest {
    pub fn path(&self) -> String {
        match self {
            Self::CreateAllowListEntry { cluster_id, .. } => {
                format!("/v2/clusters/{}/allowlist", cluster_id)
            }
            Self::CreateBucket { cluster_id, .. } => {
                format!("/v2/clusters/{}/buckets", cluster_id)
            }
            Self::CreateCluster { .. } => "/v2/clusters".into(),
            Self::CreateProject { .. } => "/v2/projects".into(),
            Self::CreateUser { cluster_id, .. } => {
                format!("/v2/clusters/{}/users", cluster_id)
            }
            Self::DeleteAllowListEntry { cluster_id, .. } => {
                format!("/v2/clouds/{}/allowlist", cluster_id)
            }
            Self::DeleteBucket { cluster_id, .. } => {
                format!("/v2/clusters/{}/buckets", cluster_id)
            }
            Self::DeleteCluster { cluster_id, .. } => {
                format!("/v2/clouds/{}", cluster_id)
            }
            Self::DeleteProject { project_id } => {
                format!("/v2/projects/{}", project_id)
            }
            Self::DeleteUser {
                cluster_id,
                username,
            } => {
                format!("/v2/clusters/{}/users/{}", cluster_id, username)
            }
            Self::GetAPIStatus => "/v2/status".into(),
            Self::GetAllowList { cluster_id } => {
                format!("/v2/clusters/{}/allowlist", cluster_id)
            }
            Self::GetBuckets { cluster_id } => {
                format!("/v2/clusters/{}/buckets", cluster_id)
            }
            Self::GetCertificate { cluster_id } => {
                format!("/v2/clusters/{}/certificate", cluster_id)
            }
            Self::GetCloud { cloud_id } => {
                format!("/v2/clouds/{}", cloud_id)
            }
            Self::GetClouds => "/v2/clouds".into(),
            Self::GetClusterHealth { cluster_id } => {
                format!("/v2/clusters/{}/health", cluster_id)
            }
            Self::GetCluster { cluster_id } => {
                format!("/v2/clusters/{}", cluster_id)
            }
            Self::GetClusters => "/v2/clusters".into(),
            Self::GetClusterStatus { cluster_id } => {
                format!("/v2/clusters/{}/status", cluster_id)
            }
            Self::GetOrgUsers => "/v2/users".into(),
            Self::GetProject { project_id } => {
                format!("/v2/projects/{}", project_id)
            }
            Self::GetProjects => "/v2/projects".into(),
            Self::GetUsers { cluster_id } => {
                format!("/v2/clusters/{}/users", cluster_id)
            }
            Self::UpdateAllowList { cluster_id, .. } => {
                format!("/v2/clusters/{}/allowlist", cluster_id)
            }
            Self::UpdateBucket { cluster_id, .. } => {
                format!("/v2/clusters/{}/buckets", cluster_id)
            }
            Self::UpdateUser { cluster_id, .. } => {
                format!("/v2/clusters/{}/users", cluster_id)
            }
        }
    }

    pub fn verb(&self) -> HttpVerb {
        match self {
            Self::CreateAllowListEntry { .. } => HttpVerb::Post,
            Self::CreateBucket { .. } => HttpVerb::Post,
            Self::CreateCluster { .. } => HttpVerb::Post,
            Self::CreateProject { .. } => HttpVerb::Post,
            Self::CreateUser { .. } => HttpVerb::Post,
            Self::DeleteAllowListEntry { .. } => HttpVerb::Delete,
            Self::DeleteBucket { .. } => HttpVerb::Delete,
            Self::DeleteCluster { .. } => HttpVerb::Delete,
            Self::DeleteProject { .. } => HttpVerb::Delete,
            Self::DeleteUser { .. } => HttpVerb::Delete,
            Self::GetAPIStatus => HttpVerb::Get,
            Self::GetAllowList { .. } => HttpVerb::Get,
            Self::GetBuckets { .. } => HttpVerb::Get,
            Self::GetCertificate { .. } => HttpVerb::Get,
            Self::GetCloud { .. } => HttpVerb::Get,
            Self::GetClouds => HttpVerb::Get,
            Self::GetClusterHealth { .. } => HttpVerb::Get,
            Self::GetCluster { .. } => HttpVerb::Get,
            Self::GetClusters => HttpVerb::Get,
            Self::GetClusterStatus { .. } => HttpVerb::Get,
            Self::GetOrgUsers => HttpVerb::Get,
            Self::GetProject { .. } => HttpVerb::Get,
            Self::GetProjects => HttpVerb::Get,
            Self::GetUsers { .. } => HttpVerb::Get,
            Self::UpdateAllowList { .. } => HttpVerb::Put,
            Self::UpdateBucket { .. } => HttpVerb::Put,
            Self::UpdateUser { .. } => HttpVerb::Put,
        }
    }

    pub fn payload(&self) -> Option<Vec<u8>> {
        match self {
            Self::CreateAllowListEntry { payload, .. } => Some(payload.as_bytes().into()),
            Self::CreateBucket { payload, .. } => Some(payload.as_bytes().into()),
            Self::CreateCluster { payload, .. } => Some(payload.as_bytes().into()),
            Self::CreateProject { payload, .. } => Some(payload.as_bytes().into()),
            Self::CreateUser { payload, .. } => Some(payload.as_bytes().into()),
            Self::UpdateAllowList { payload, .. } => Some(payload.as_bytes().into()),
            Self::UpdateBucket { payload, .. } => Some(payload.as_bytes().into()),
            Self::UpdateUser { payload, .. } => Some(payload.as_bytes().into()),
            _ => None,
        }
    }

    pub fn headers(&self) -> HashMap<&str, &str> {
        match self {
            Self::CreateAllowListEntry { .. } => {
                let mut h = HashMap::new();
                h.insert("Content-Type", "application/json");
                h
            }
            Self::CreateBucket { .. } => {
                let mut h = HashMap::new();
                h.insert("Content-Type", "application/json");
                h
            }
            Self::CreateCluster { .. } => {
                let mut h = HashMap::new();
                h.insert("Content-Type", "application/json");
                h
            }
            Self::CreateProject { .. } => {
                let mut h = HashMap::new();
                h.insert("Content-Type", "application/json");
                h
            }
            Self::CreateUser { .. } => {
                let mut h = HashMap::new();
                h.insert("Content-Type", "application/json");
                h
            }
            Self::UpdateAllowList { .. } => {
                let mut h = HashMap::new();
                h.insert("Content-Type", "application/json");
                h
            }
            Self::UpdateBucket { .. } => {
                let mut h = HashMap::new();
                h.insert("Content-Type", "application/json");
                h
            }
            Self::UpdateUser { .. } => {
                let mut h = HashMap::new();
                h.insert("Content-Type", "application/json");
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
        }
    }
}

pub enum AnalyticsQueryRequest {
    Execute {
        statement: String,
        scope: Option<(String, String)>,
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
pub struct PingResponse {
    state: String,
    address: String,
    service: ServiceType,
    latency: Duration,
    error: Option<ClientError>,
}

impl PingResponse {
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

#[derive(Debug)]
pub struct KvResponse {
    content: Option<serde_json::Value>,
    cas: u64,
}

impl KvResponse {
    pub fn content(&mut self) -> Option<serde_json::Value> {
        self.content.take()
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

    pub fn view_seeds(&self, tls: bool) -> Vec<(String, u32)> {
        let key = if tls { "capiSSL" } else { "capi" };

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
                (hostname, *node.services.get(key).unwrap())
            })
            .collect()
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

struct CollectionDetails {
    scope: String,
    collection: String,
    manifest: CollectionManifest,
}

// Thinking here that some of this will need to go into arc mutexes at some point.
pub struct KvClient {
    username: String,
    password: String,
    collection: Option<CollectionDetails>,
    endpoints: HashMap<String, KvEndpoint>,
    config: BucketConfig,
    tls_config: ClusterTlsConfig,
    bucket: String,
}

impl KvClient {
    fn partition_for_key(&self, key: String, config: &BucketConfig) -> u32 {
        let num_partitions = config.vbucket_server_map.vbucket_map.len() as u32;

        let sum = (crc32::checksum_ieee(key.as_bytes()) >> 16) & 0x7fff;
        sum % num_partitions
    }

    fn node_for_partition(&self, partition: u32, config: &BucketConfig) -> (String, u32) {
        let seeds = config.key_value_seeds(self.tls_config.enabled());
        let node = config.vbucket_server_map.vbucket_map[partition as usize][0];

        let seed = &seeds[node as usize];
        let addr = seed.0.clone();
        let port = seed.1;

        (addr, port)
    }

    fn search_manifest(
        &self,
        scope: String,
        collection: String,
        manifest: &CollectionManifest,
    ) -> Result<u32, ClientError> {
        for s in &manifest.scopes {
            if s.name == scope {
                for c in &s.collections {
                    if c.name == collection {
                        return Ok(c.uid.parse::<u32>().unwrap());
                    }
                }
            }
        }
        Err(CollectionNotFound)
    }

    pub async fn ping_all(
        &mut self,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<Vec<PingResponse>, ClientError> {
        let now = Instant::now();
        if now >= deadline {
            return Err(ClientError::Timeout);
        }
        let deadline_sleep = sleep(deadline.sub(now));
        tokio::pin!(deadline_sleep);

        let ctrl_c_fut = CtrlcFuture::new(ctrl_c);
        tokio::pin!(ctrl_c_fut);

        let mut results: Vec<PingResponse> = Vec::new();
        for seed in self.config.key_value_seeds(self.tls_config.enabled()) {
            let addr = seed.0;
            let port = seed.1;
            let mut ep = self.endpoints.get(addr.clone().as_str());
            if ep.is_none() {
                let connect = KvEndpoint::connect(
                    addr.clone(),
                    port,
                    self.username.clone(),
                    self.password.clone(),
                    self.bucket.clone(),
                    self.tls_config.clone(),
                );

                let endpoint = select! {
                    res = connect => res,
                    () = &mut deadline_sleep => Err(ClientError::Timeout),
                    () = &mut ctrl_c_fut => Err(ClientError::Cancelled),
                }?;

                // Got to be a better way...
                self.endpoints.insert(addr.clone(), endpoint);
                ep = self.endpoints.get(addr.clone().as_str());
            };

            let op = ep.unwrap().noop();

            let start = Instant::now();
            let result = select! {
                res = op => res,
                () = &mut deadline_sleep => Err(ClientError::Timeout),
                () = &mut ctrl_c_fut => Err(ClientError::Cancelled),
            };
            let end = Instant::now();

            let error = match result {
                Ok(_) => None,
                Err(e) => Some(e),
            };

            let mut state = "OK".into();
            if error.is_some() {
                state = "Error".into();
            }

            results.push(PingResponse {
                address: format!("{}:{}", addr.clone(), port.clone()),
                service: ServiceType::KeyValue,
                state,
                error,
                latency: end.sub(start),
            });
        }

        Ok(results)
    }

    pub async fn request(
        &mut self,
        request: KeyValueRequest,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<KvResponse, ClientError> {
        let now = Instant::now();
        if now >= deadline {
            return Err(ClientError::Timeout);
        }
        let deadline_sleep = sleep(deadline.sub(now));
        tokio::pin!(deadline_sleep);

        let ctrl_c_fut = CtrlcFuture::new(ctrl_c);
        tokio::pin!(ctrl_c_fut);

        let cid = if let Some(collection) = self.collection.as_ref() {
            self.search_manifest(
                collection.scope.clone(),
                collection.collection.clone(),
                &collection.manifest,
            )?
        } else {
            0
        };

        let key = match request {
            KeyValueRequest::Get { ref key } => key.clone(),
            KeyValueRequest::Set { ref key, .. } => key.clone(),
            KeyValueRequest::Insert { ref key, .. } => key.clone(),
            KeyValueRequest::Replace { ref key, .. } => key.clone(),
            KeyValueRequest::Remove { ref key, .. } => key.clone(),
        };

        let config = &self.config;
        let partition = self.partition_for_key(key.clone(), config);
        let (addr, port) = self.node_for_partition(partition, config);

        let mut ep = self.endpoints.get(addr.clone().as_str());
        if ep.is_none() {
            let connect = KvEndpoint::connect(
                addr.clone(),
                port,
                self.username.clone(),
                self.password.clone(),
                self.bucket.clone(),
                self.tls_config.clone(),
            );

            let endpoint = select! {
                res = connect => res,
                () = &mut deadline_sleep => Err(ClientError::Timeout),
                () = &mut ctrl_c_fut => Err(ClientError::Cancelled),
            }?;

            // Got to be a better way...
            self.endpoints.insert(addr.clone(), endpoint);
            ep = self.endpoints.get(addr.clone().as_str());
        }

        let result = match request {
            KeyValueRequest::Get { key } => {
                // ep cannot be None so unwrap is safe to do.
                let op = ep.unwrap().get(key.clone(), partition as u16, cid);

                select! {
                    res = op => res,
                    () = &mut deadline_sleep => Err(ClientError::Timeout),
                    () = &mut ctrl_c_fut => Err(ClientError::Cancelled),
                }
            }
            KeyValueRequest::Set { key, value, expiry } => {
                // ep cannot be None so unwrap is safe to do.
                let op = ep
                    .unwrap()
                    .set(key.clone(), value, expiry, partition as u16, cid);

                select! {
                    res = op => res,
                    () = &mut deadline_sleep => Err(ClientError::Timeout),
                    () = &mut ctrl_c_fut => Err(ClientError::Cancelled),
                }
            }
            KeyValueRequest::Insert { key, value, expiry } => {
                // ep cannot be None so unwrap is safe to do.
                let op = ep
                    .unwrap()
                    .add(key.clone(), value, expiry, partition as u16, cid);

                select! {
                    res = op => res,
                    () = &mut deadline_sleep => Err(ClientError::Timeout),
                    () = &mut ctrl_c_fut => Err(ClientError::Cancelled),
                }
            }
            KeyValueRequest::Replace { key, value, expiry } => {
                // ep cannot be None so unwrap is safe to do.
                let op = ep
                    .unwrap()
                    .replace(key.clone(), value, expiry, partition as u16, cid);

                select! {
                    res = op => res,
                    () = &mut deadline_sleep => Err(ClientError::Timeout),
                    () = &mut ctrl_c_fut => Err(ClientError::Cancelled),
                }
            }
            KeyValueRequest::Remove { key } => {
                // ep cannot be None so unwrap is safe to do.
                let op = ep.unwrap().remove(key.clone(), partition as u16, cid);

                select! {
                    res = op => res,
                    () = &mut deadline_sleep => Err(ClientError::Timeout),
                    () = &mut ctrl_c_fut => Err(ClientError::Cancelled),
                }
            }
        };

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
                    content,
                    cas: r.cas(),
                })
            }
            Err(e) => Err(e),
        }
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
                (hostname, *node.services.get(key).unwrap())
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
    Get {
        key: String,
    },
    Set {
        key: String,
        value: Vec<u8>,
        expiry: u32,
    },
    Insert {
        key: String,
        value: Vec<u8>,
        expiry: u32,
    },
    Replace {
        key: String,
        value: Vec<u8>,
        expiry: u32,
    },
    Remove {
        key: String,
    },
}
