use crate::cli::CtrlcFuture;
use crate::client::error::ClientError;
use crate::client::http_client::{Config, PingResponse, ServiceType};
use crate::client::http_handler::HTTPHandler;
use crate::client::kv::KvEndpoint;
use crate::client::HTTPClient;
use crate::RustTlsConfig;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use log::debug;
use nu_protocol::Signals;
use serde::Deserialize;
use std::{collections::HashMap, ops::Sub};
use tokio::select;
use tokio::time::{sleep, Instant};

#[derive(Debug)]
pub struct KvResponse {
    pub(crate) content: Option<serde_json::Value>,
    pub(crate) cas: u64,
    pub(crate) key: String,
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
    endpoints: HashMap<String, KvEndpoint>,
    config: BucketConfig,
    tls_enabled: bool,
}

impl KvClient {
    pub async fn connect(
        seeds: Vec<String>,
        username: String,
        password: String,
        tls_config: Option<RustTlsConfig>,
        bucket: String,
        deadline: Instant,
        signals: Signals,
    ) -> Result<Self, ClientError> {
        let now = Instant::now();
        if now >= deadline {
            return Err(ClientError::Timeout { key: None });
        }
        let deadline_sleep = sleep(deadline.sub(now));
        tokio::pin!(deadline_sleep);

        let ctrlc_fut = CtrlcFuture::new(signals.clone());
        tokio::pin!(ctrlc_fut);

        let http_agent = HTTPHandler::new(username.clone(), password.clone(), tls_config.clone());
        let config: BucketConfig = HTTPClient::get_config(
            &seeds,
            tls_config.is_some(),
            &http_agent,
            bucket.clone(),
            deadline,
            signals.clone(),
        )
        .await?;

        let mut workers = FuturesUnordered::new();
        for addr in config.key_value_seeds(tls_config.is_some()) {
            let hostname = addr.0.clone();
            let port = addr.1;
            let u = username.clone();
            let p = password.clone();
            let b = bucket.clone();
            let tls = tls_config.clone();

            workers.push(tokio::spawn(async move {
                KvEndpoint::connect(hostname, port, u, p, b, tls).await
            }));
        }

        let mut endpoints = HashMap::new();
        loop {
            let endpoint = select! {
                res = workers.next() => {
                    match res {
                        Some(ep) => {
                            // The top level result is a result containing what we want and a JoinError which
                            // occurs if the future panics.
                            match ep {
                                Ok(r) => r,
                                Err(e) => Err(ClientError::RequestFailed {reason: Some(e.to_string()), key: None}),
                            }
                        },
                        None => break
                    }
                },
                () = &mut deadline_sleep => Err(ClientError::Timeout{key: None}),
                () = &mut ctrlc_fut => Err(ClientError::Cancelled{key: None}),
                else => {break}
            }?;
            endpoints.insert(endpoint.remote(), endpoint);
        }

        Ok(Self {
            config,
            endpoints,
            tls_enabled: tls_config.is_some(),
        })
    }

    pub async fn ping_all(
        &mut self,
        deadline: Instant,
        signals: Signals,
    ) -> Result<Vec<PingResponse>, ClientError> {
        let now = Instant::now();
        if now >= deadline {
            return Err(ClientError::Timeout { key: None });
        }
        let deadline_sleep = sleep(deadline.sub(now));
        tokio::pin!(deadline_sleep);
        tokio::pin!(deadline_sleep);

        let ctrlc_fut = CtrlcFuture::new(signals.clone());
        tokio::pin!(ctrlc_fut);

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
                () = &mut ctrlc_fut => Err(ClientError::Cancelled{key: None}),
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
}

#[derive(Deserialize, Debug)]
struct BucketConfig {
    // rev: u64,
    #[serde(alias = "nodesExt")]
    nodes_ext: Vec<NodeExtConfig>,
    nodes: Vec<NodeConfig>,
    loaded_from: Option<String>,
}

impl Config for BucketConfig {
    fn set_loaded_from(&mut self, loaded_from: String) {
        self.loaded_from = Some(loaded_from);
    }
}

impl BucketConfig {
    pub fn key_value_seeds(&self, tls: bool) -> Vec<(String, u32)> {
        let key = if tls { "kvSSL" } else { "kv" };

        self.seeds(key)
    }

    fn seeds(&self, key: &str) -> Vec<(String, u32)> {
        let len_nodes = self.nodes.len();
        let default: Vec<(String, u32)> = self
            .nodes_ext
            .iter()
            .enumerate()
            .filter(|&(i, node)| {
                if i >= len_nodes {
                    let hostname = if node.hostname.is_some() {
                        node.hostname.as_ref().unwrap().clone()
                    } else {
                        self.loaded_from.as_ref().unwrap().clone()
                    };
                    debug!(
                        "Node {} present in nodes ext but not in nodes, skipping",
                        hostname
                    );
                    return false;
                }
                node.services.contains_key(key)
            })
            .map(|node| {
                let hostname = if node.1.hostname.is_some() {
                    node.1.hostname.as_ref().unwrap().clone()
                } else {
                    self.loaded_from.as_ref().unwrap().clone()
                };
                (hostname, *node.1.services.get(key).unwrap())
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
pub(crate) struct NodeExtConfig {
    pub(crate) services: HashMap<String, u32>,
    // #[serde(alias = "thisNode")]
    // pub(crate) this_node: Option<bool>,
    pub(crate) hostname: Option<String>,
    #[serde(alias = "alternateAddresses", default)]
    pub(crate) alternate_addresses: HashMap<String, AlternateAddress>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct NodeConfig {
    // pub(crate) hostname: Option<String>,
    // #[serde(default)]
    // pub(crate) ports: HashMap<String, i32>,
}
