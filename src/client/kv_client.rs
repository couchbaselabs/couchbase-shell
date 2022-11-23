use crate::cli::CtrlcFuture;
use crate::client::crc::cb_vb_map;
use crate::client::error::ClientError;
use crate::client::http_client::{PingResponse, ServiceType};
use crate::client::http_handler::{status_to_reason, HTTPHandler};
use crate::client::kv::{KvEndpoint, KvTlsConfig};
use crate::client::protocol;
use crate::config::ClusterTlsConfig;
use bytes::{Buf, Bytes};
use log::{debug, trace};
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
    extras: Option<Bytes>,
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

    pub fn extras(&mut self) -> Option<Bytes> {
        self.extras.take()
    }
}

pub struct KvClient {
    endpoints: HashMap<String, KvEndpoint>,
    config: BucketConfig,
    tls_enabled: bool,
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
            seeds,
            bucket.clone(),
            tls_config.clone(),
            &http_agent,
            deadline,
            ctrl_c.clone(),
        )
        .await?;

        let tls_enabled = tls_config.enabled();
        let kv_tls_config = if tls_enabled {
            Some(KvTlsConfig::new(tls_config)?)
        } else {
            None
        };

        let mut endpoints = HashMap::new();
        for addr in config.key_value_seeds(tls_enabled) {
            let connect = KvEndpoint::connect(
                addr.0.clone(),
                addr.1,
                username.clone(),
                password.clone(),
                bucket.clone(),
                kv_tls_config.clone(),
            );

            let endpoint = select! {
                res = connect => res,
                () = &mut deadline_sleep => Err(ClientError::Timeout{key: None}),
                () = &mut ctrl_c_fut => Err(ClientError::Cancelled{key: None}),
            }?;

            endpoints.insert(format!("{}:{}", addr.0, addr.1), endpoint);
        }

        Ok(Self {
            config,
            endpoints,
            tls_enabled: kv_tls_config.is_some(),
        })
    }

    fn partition_for_key(&self, key: String) -> u32 {
        let num_partitions = self.config.vbucket_server_map.vbucket_map.len() as u32;

        cb_vb_map(key.as_bytes().to_vec(), num_partitions)
    }

    fn node_for_partition(&self, partition: u32) -> (String, u32) {
        let seeds = self.config.key_value_seeds(self.tls_enabled);
        let node = self.config.vbucket_server_map.vbucket_map[partition as usize][0];

        let seed = &seeds[node as usize];
        let addr = seed.0.clone();
        let port = seed.1;

        (addr, port)
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

            let uri = format!("{}:{}{}", host, port, &path);
            debug!("Fetching config from {}", uri);
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

            trace!("Fetched config {:?}", &config);
            return Ok(config);
        }
        let mut reason = final_error_content;
        if reason.is_none() {
            reason = status_to_reason(final_error_status);
        }
        Err(ClientError::ConfigurationLoadFailed { reason })
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
        for seed in self.config.key_value_seeds(self.tls_enabled) {
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

            let mut state = "OK".to_string();
            if error.is_some() {
                state = "Error".to_string();
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

    pub fn is_non_default_scope_collection(scope: String, collection: String) -> bool {
        (!scope.is_empty() && scope != "_default")
            || (!collection.is_empty() && collection != "_default")
    }

    pub async fn request(
        &self,
        request: KeyValueRequest,
        cid: u32,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<KvResponse, ClientError> {
        let now = Instant::now();
        if now >= deadline {
            return Err(ClientError::Timeout {
                key: Some(request.key()),
            });
        }
        let deadline = deadline.sub(now);
        let deadline_sleep = sleep(deadline);
        tokio::pin!(deadline_sleep);

        let ctrl_c_fut = CtrlcFuture::new(ctrl_c.clone());
        tokio::pin!(ctrl_c_fut);

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

        self.handle_op_result(result)
    }

    fn handle_op_result(
        &self,
        result: Result<(protocol::KvResponse, Option<String>), ClientError>,
    ) -> Result<KvResponse, ClientError> {
        match result {
            Ok(mut r) => {
                let content = if let Some(body) = r.0.body() {
                    match serde_json::from_slice(body.as_ref()) {
                        Ok(v) => Some(v),
                        Err(e) => {
                            return Err(ClientError::RequestFailed {
                                reason: Some(e.to_string()),
                                key: r.1,
                            });
                        }
                    }
                } else {
                    None
                };
                Ok(KvResponse {
                    content,
                    cas: r.0.cas(),
                    key: r.1.unwrap_or_default(),
                    extras: r.0.extras(),
                })
            }
            Err(e) => Err(e),
        }
    }

    pub async fn get_cid(
        &self,
        scope: String,
        collection: String,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<u32, ClientError> {
        if !KvClient::is_non_default_scope_collection(scope.clone(), collection.clone()) {
            trace!(
                "Scope and collection names both empty or _default, not performing manifest lookup"
            );
            return Ok(0);
        }

        let scope_name = if scope.is_empty() {
            trace!("Coerced empty scope name to _default");
            "_default".to_string()
        } else {
            scope
        };
        let collection_name = if collection.is_empty() {
            trace!("Coerced empty collection name to _default");
            "_default".to_string()
        } else {
            collection
        };

        let deadline_sleep = sleep(deadline.sub(Instant::now()));
        tokio::pin!(deadline_sleep);

        let ctrl_c_fut = CtrlcFuture::new(ctrl_c.clone());
        tokio::pin!(ctrl_c_fut);

        let (addr, port) = self.node_for_partition(0);
        let ep = self
            .endpoints
            .get(format!("{}:{}", addr.clone(), port).as_str())
            .unwrap();

        let op = ep.get_cid(scope_name, collection_name);

        let resp = self
            .handle_op_future(None, op, deadline_sleep, ctrl_c_fut)
            .await;

        let mut result = self.handle_op_result(resp)?;
        match result.extras() {
            Some(mut e) => {
                if e.len() < 12 {
                    return Err(ClientError::RequestFailed {
                        reason: Some(
                            "Response from get collection id not expected format".to_string(),
                        ),
                        key: None,
                    });
                }
                // Skip over the manifest uid
                e.advance(8);
                Ok(e.get_u32())
            }
            None => Err(ClientError::RequestFailed {
                reason: Some("Response from get collection id not expected format".to_string()),
                key: None,
            }),
        }
    }

    // handle_op_future resolves the future into a result containing (response, key) or an error.
    async fn handle_op_future(
        &self,
        key: impl Into<Option<String>>,
        op: impl Future<Output = Result<protocol::KvResponse, ClientError>>,
        mut deadline_sleep: Pin<&mut Sleep>,
        mut ctrl_c: Pin<&mut CtrlcFuture>,
    ) -> Result<(protocol::KvResponse, Option<String>), ClientError> {
        let key = key.into();
        let res = select! {
            res = op => res,
            () = &mut deadline_sleep => Err(ClientError::Timeout{key: key.clone()}),
            () = &mut ctrl_c => Err(ClientError::Cancelled{key: key.clone()}),
        }?;

        Ok((res, key))
    }
}

#[derive(Deserialize, Debug)]
struct BucketConfig {
    // rev: u64,
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
    // #[serde(alias = "thisNode")]
    // pub(crate) this_node: Option<bool>,
    pub(crate) hostname: Option<String>,
    #[serde(alias = "alternateAddresses", default)]
    pub(crate) alternate_addresses: HashMap<String, AlternateAddress>,
}

#[derive(Deserialize, Debug)]
struct VBucketServerMap {
    // #[serde(alias = "numReplicas")]
    // num_replicas: u32,
    // #[serde(alias = "serverList")]
    // server_list: Vec<String>,
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
