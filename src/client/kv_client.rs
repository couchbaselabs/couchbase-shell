use crate::cli::CtrlcFuture;
use crate::client::error::ClientError;
use crate::client::error::ClientError::CollectionNotFound;
use crate::client::http_client::{PingResponse, ServiceType};
use crate::client::http_handler::{http_prefix, status_to_reason, HTTPHandler};
use crate::client::kv::KvEndpoint;
use crate::config::ClusterTlsConfig;
use crc::crc32;
use serde::Deserialize;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::{collections::HashMap, ops::Sub};
use tokio::runtime::Runtime;
use tokio::time::sleep;
use tokio::{select, time::Instant};

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

// Thinking here that some of this will need to go into arc mutexes at some point.
pub struct KvClient {
    seeds: Vec<String>,
    username: String,
    password: String,
    manifest: Option<CollectionManifest>,
    endpoints: HashMap<String, KvEndpoint>,
    config: Option<BucketConfig>,
    tls_config: ClusterTlsConfig,
    http_agent: HTTPHandler,
}

impl KvClient {
    pub fn new(
        seeds: Vec<String>,
        username: String,
        password: String,
        tls_config: ClusterTlsConfig,
    ) -> Self {
        Self {
            seeds,
            username: username.clone(),
            password: password.clone(),
            manifest: None,
            config: None,
            endpoints: HashMap::new(),
            tls_config: tls_config.clone(),
            http_agent: HTTPHandler::new(username, password, tls_config),
        }
    }

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

    fn search_manifest(&self, scope: String, collection: String) -> Result<u32, ClientError> {
        for s in &self.manifest.as_ref().unwrap().scopes {
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

    async fn get_bucket_config(
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
            let uri = format!(
                "{}://{}:{}{}",
                http_prefix(&self.tls_config),
                seed,
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

    pub fn ping_all(
        &mut self,
        bucket: String,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<Vec<PingResponse>, ClientError> {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let now = Instant::now();
            if now >= deadline {
                return Err(ClientError::Timeout);
            }
            let deadline_sleep = sleep(deadline.sub(now));
            tokio::pin!(deadline_sleep);

            let ctrl_c_fut = CtrlcFuture::new(ctrl_c.clone());
            tokio::pin!(ctrl_c_fut);

            if self.config.is_none() {
                self.config = Some(
                    self.get_bucket_config(bucket.clone(), deadline, ctrl_c.clone())
                        .await?,
                );
            }

            let mut results: Vec<PingResponse> = Vec::new();
            for seed in self
                .config
                .as_ref()
                .unwrap()
                .key_value_seeds(self.tls_config.enabled())
            {
                let addr = seed.0.clone();
                let port = seed.1;
                let mut ep = self.endpoints.get(addr.clone().as_str());
                if ep.is_none() {
                    let connect = KvEndpoint::connect(
                        addr.clone(),
                        port,
                        self.username.clone(),
                        self.password.clone(),
                        bucket.clone(),
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

                results.push(PingResponse::new(
                    state,
                    format!("{}:{}", addr.clone(), port.clone()),
                    ServiceType::KeyValue,
                    end.sub(start),
                    error,
                ));
            }

            Ok(results)
        })
    }

    pub fn request(
        &mut self,
        request: KeyValueRequest,
        bucket: String,
        scope: String,
        collection: String,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<KvResponse, ClientError> {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let now = Instant::now();
            if now >= deadline {
                return Err(ClientError::Timeout);
            }
            let deadline_sleep = sleep(deadline.sub(now));
            tokio::pin!(deadline_sleep);

            let ctrl_c_fut = CtrlcFuture::new(ctrl_c.clone());
            tokio::pin!(ctrl_c_fut);

            if self.config.is_none() {
                self.config = Some(
                    self.get_bucket_config(bucket.clone(), deadline, ctrl_c.clone())
                        .await?,
                );
            }

            let cid = if (!scope.is_empty() && scope != "_default")
                || (!collection.is_empty() && collection != "_default")
            {
                if self.manifest.is_none() {
                    // If we've been specifically asked to use a scope or collection and fetching the manifest
                    // fails then we need to report that.
                    self.manifest = Some(
                        self.get_collection_manifest(bucket.clone(), deadline, ctrl_c.clone())
                            .await?,
                    );
                }

                self.search_manifest(scope.clone(), collection.clone())?
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

            let config = self.config.as_ref().unwrap();
            let partition = self.partition_for_key(key.clone(), config);
            let (addr, port) = self.node_for_partition(partition, config);

            let mut ep = self.endpoints.get(addr.clone().as_str());
            if ep.is_none() {
                let connect = KvEndpoint::connect(
                    addr.clone(),
                    port,
                    self.username.clone(),
                    self.password.clone(),
                    bucket,
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
        })
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
