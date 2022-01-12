use crate::cli::CtrlcFuture;
use crate::client::error::ClientError;
use crate::client::error::ClientError::CollectionNotFound;
use crate::client::http_client::{PingResponse, ServiceType};
use crate::client::http_handler::{http_prefix, status_to_reason, HTTPHandler};
use crate::client::kv::KvEndpoint;
use crate::client::protocol;
use crate::config::ClusterTlsConfig;
use crc::crc32;
use serde::Deserialize;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::{collections::HashMap, ops::Sub};
use tokio::select;
use tokio::time::{sleep, Instant, Sleep};

#[derive(Debug)]
pub struct KvResponse {
    content: Option<serde_json::Value>,
    cas: u64,
    key: String,
}

impl KvResponse {
    pub fn content(&mut self) -> Option<serde_json::Value> {
        self.content.take()
    }

    pub fn cas(&self) -> u64 {
        self.cas
    }

    pub fn key(&self) -> String {
        self.key.clone()
    }
}

pub struct KvClient {
    seeds: Vec<String>,
    manifest: Option<CollectionManifest>,
    endpoints: HashMap<String, KvEndpoint>,
    config: BucketConfig,
    tls_config: ClusterTlsConfig,
    http_agent: HTTPHandler,
    bucket: String,
}

impl KvClient {
    pub async fn connect(
        seeds: Vec<String>,
        username: String,
        password: String,
        tls_config: ClusterTlsConfig,
        bucket: String,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<Self, ClientError> {
        let now = Instant::now();
        if now >= deadline {
            return Err(ClientError::Timeout { key: None });
        }
        let deadline_sleep = sleep(deadline.sub(now));
        tokio::pin!(deadline_sleep);

        let ctrl_c_fut = CtrlcFuture::new(ctrl_c.clone());
        tokio::pin!(ctrl_c_fut);

        let http_agent = HTTPHandler::new(username.clone(), password.clone(), tls_config.clone());
        let config = KvClient::get_bucket_config(
            seeds.clone(),
            bucket.clone(),
            tls_config.clone(),
            &http_agent,
            deadline,
            ctrl_c.clone(),
        )
        .await?;

        let mut endpoints = HashMap::new();
        for addr in config.key_value_seeds(tls_config.enabled()) {
            let connect = KvEndpoint::connect(
                addr.0.clone(),
                addr.1,
                username.clone(),
                password.clone(),
                bucket.clone(),
                tls_config.clone(),
            );

            let endpoint = select! {
                res = connect => res,
                () = &mut deadline_sleep => Err(ClientError::Timeout{key: None}),
                () = &mut ctrl_c_fut => Err(ClientError::Cancelled{key: None}),
            }?;

            endpoints.insert(format!("{}:{}", addr.0, addr.1), endpoint);
        }

        Ok(Self {
            seeds,
            manifest: None,
            config,
            endpoints,
            tls_config: tls_config.clone(),
            http_agent,
            bucket,
        })
    }

    fn partition_for_key(&self, key: String) -> u32 {
        let num_partitions = self.config.vbucket_server_map.vbucket_map.len() as u32;

        let sum = (crc32::checksum_ieee(key.as_bytes()) >> 16) & 0x7fff;
        sum % num_partitions
    }

    fn node_for_partition(&self, partition: u32) -> (String, u32) {
        let seeds = self.config.key_value_seeds(self.tls_config.enabled());
        let node = self.config.vbucket_server_map.vbucket_map[partition as usize][0];

        let seed = &seeds[node as usize];
        let addr = seed.0.clone();
        let port = seed.1;

        (addr, port)
    }

    fn search_manifest(
        &self,
        key: String,
        scope: String,
        collection: String,
    ) -> Result<u32, ClientError> {
        for s in &self.manifest.as_ref().unwrap().scopes {
            if s.name == scope {
                for c in &s.collections {
                    if c.name == collection {
                        return Ok(u32::from_str_radix(c.uid.as_str(), 16).unwrap());
                    }
                }
            }
        }
        Err(CollectionNotFound { key: Some(key) })
    }

    async fn get_bucket_config(
        seeds: Vec<String>,
        bucket: String,
        tls_config: ClusterTlsConfig,
        http_agent: &HTTPHandler,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<BucketConfig, ClientError> {
        let path = format!("/pools/default/b/{}", bucket);
        let mut final_error_content = None;
        let mut final_error_status = 0;
        for seed in seeds {
            let host_split: Vec<String> = seed.split(':').map(|v| v.to_owned()).collect();

            let host: String;
            let port: i32;
            if host_split.len() == 1 {
                host = seed.clone();
                port = if tls_config.enabled() { 18091 } else { 8091 };
            } else {
                host = host_split[0].clone();
                port = host_split[1]
                    .parse::<i32>()
                    .map_err(|e| ClientError::RequestFailed {
                        reason: Some(e.to_string()),
                        key: None,
                    })?;
            }

            let uri = format!("{}://{}:{}{}", http_prefix(&tls_config), host, port, &path);
            let (content, status) = http_agent.http_get(&uri, deadline, ctrl_c.clone()).await?;
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
            reason = status_to_reason(final_error_status);
        }
        Err(ClientError::ConfigurationLoadFailed { reason })
    }

    async fn get_collection_manifest(
        &self,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<CollectionManifest, ClientError> {
        let path = format!("/pools/default/buckets/{}/scopes/", self.bucket.clone());
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
                .http_agent
                .http_get(&uri, deadline, ctrl_c.clone())
                .await?;
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
            reason = status_to_reason(final_error_status);
        }
        Err(ClientError::CollectionManifestLoadFailed { reason })
    }

    pub async fn ping_all(
        &mut self,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<Vec<PingResponse>, ClientError> {
        let now = Instant::now();
        if now >= deadline {
            return Err(ClientError::Timeout { key: None });
        }
        let deadline_sleep = sleep(deadline.sub(now));
        tokio::pin!(deadline_sleep);
        tokio::pin!(deadline_sleep);

        let ctrl_c_fut = CtrlcFuture::new(ctrl_c.clone());
        tokio::pin!(ctrl_c_fut);

        let mut results: Vec<PingResponse> = Vec::new();
        for seed in self.config.key_value_seeds(self.tls_config.enabled()) {
            let addr = seed.0.clone();
            let port = seed.1;
            let ep = self
                .endpoints
                .get(format!("{}:{}", addr.clone(), port).as_str())
                .unwrap();

            let op = ep.noop();

            let start = Instant::now();
            let result = select! {
                res = op => res,
                () = &mut deadline_sleep => Err(ClientError::Timeout{key: None}),
                () = &mut ctrl_c_fut => Err(ClientError::Cancelled{key: None}),
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

            results.push(PingResponse::new(
                state,
                format!("{}:{}", addr.clone(), port.clone()),
                ServiceType::KeyValue,
                end.sub(start),
                error,
            ));
        }

        Ok(results)
    }

    pub async fn fetch_collections_manifest(
        &mut self,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<(), ClientError> {
        let now = Instant::now();
        if now >= deadline {
            return Err(ClientError::Timeout { key: None });
        }

        self.manifest = Some(
            self.get_collection_manifest(deadline, ctrl_c.clone())
                .await?,
        );

        Ok(())
    }

    pub fn is_non_default_scope_collection(scope: String, collection: String) -> bool {
        (!scope.is_empty() && scope != "_default")
            || (!collection.is_empty() && collection != "_default")
    }

    pub async fn request(
        &self,
        request: KeyValueRequest,
        scope: String,
        collection: String,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<KvResponse, ClientError> {
        let now = Instant::now();
        if now >= deadline {
            return Err(ClientError::Timeout {
                key: Some(request.key()),
            });
        }
        let deadline_sleep = sleep(deadline.sub(now));
        tokio::pin!(deadline_sleep);

        let ctrl_c_fut = CtrlcFuture::new(ctrl_c.clone());
        tokio::pin!(ctrl_c_fut);

        let cid = if (!scope.is_empty() && scope != "_default")
            || (!collection.is_empty() && collection != "_default")
        {
            self.search_manifest(request.key(), scope.clone(), collection.clone())?
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

        let partition = self.partition_for_key(key.clone());
        let (addr, port) = self.node_for_partition(partition);

        let ep = self
            .endpoints
            .get(format!("{}:{}", addr.clone(), port).as_str())
            .unwrap();

        let result = match request {
            KeyValueRequest::Get { key } => {
                let op = ep.get(key.clone(), partition as u16, cid);

                self.handle_op_future(key, op, deadline_sleep, ctrl_c_fut)
                    .await
            }
            KeyValueRequest::Set { key, value, expiry } => {
                let op = ep.set(key.clone(), value, expiry, partition as u16, cid);

                self.handle_op_future(key, op, deadline_sleep, ctrl_c_fut)
                    .await
            }
            KeyValueRequest::Insert { key, value, expiry } => {
                let op = ep.add(key.clone(), value, expiry, partition as u16, cid);

                self.handle_op_future(key, op, deadline_sleep, ctrl_c_fut)
                    .await
            }
            KeyValueRequest::Replace { key, value, expiry } => {
                let op = ep.replace(key.clone(), value, expiry, partition as u16, cid);

                self.handle_op_future(key, op, deadline_sleep, ctrl_c_fut)
                    .await
            }
            KeyValueRequest::Remove { key } => {
                let op = ep.remove(key.clone(), partition as u16, cid);

                self.handle_op_future(key, op, deadline_sleep, ctrl_c_fut)
                    .await
            }
        };

        match result {
            Ok(mut r) => {
                let content = if let Some(body) = r.0.body() {
                    match serde_json::from_slice(body.as_ref()) {
                        Ok(v) => Some(v),
                        Err(e) => {
                            return Err(ClientError::RequestFailed {
                                reason: Some(e.to_string()),
                                key: Some(key),
                            });
                        }
                    }
                } else {
                    None
                };
                Ok(KvResponse {
                    content,
                    cas: r.0.cas(),
                    key: r.1,
                })
            }
            Err(e) => Err(e),
        }
    }

    async fn handle_op_future(
        &self,
        key: String,
        op: impl Future<Output = Result<protocol::KvResponse, ClientError>>,
        mut deadline_sleep: Pin<&mut Sleep>,
        mut ctrl_c: Pin<&mut CtrlcFuture>,
    ) -> Result<(protocol::KvResponse, String), ClientError> {
        let res = select! {
            res = op => res,
            () = &mut deadline_sleep => Err(ClientError::Timeout{key: Some(key.clone())}),
            () = &mut ctrl_c => Err(ClientError::Cancelled{key: Some(key.clone())}),
        };

        match res {
            Ok(r) => Ok((r, key)),
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
}

#[derive(Deserialize, Debug)]
pub(crate) struct AlternateAddress {
    pub(crate) hostname: Option<String>,
    pub(crate) ports: HashMap<String, u32>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct NodeConfig {
    pub(crate) services: HashMap<String, u32>,
    #[serde(alias = "thisNode")]
    pub(crate) this_node: Option<bool>,
    pub(crate) hostname: Option<String>,
    #[serde(alias = "alternateAddresses", default)]
    pub(crate) alternate_addresses: HashMap<String, AlternateAddress>,
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

impl KeyValueRequest {
    pub fn key(&self) -> String {
        match self {
            KeyValueRequest::Get { key } => key.clone(),
            KeyValueRequest::Set { key, .. } => key.clone(),
            KeyValueRequest::Insert { key, .. } => key.clone(),
            KeyValueRequest::Replace { key, .. } => key.clone(),
            KeyValueRequest::Remove { key } => key.clone(),
        }
    }
}
