pub use crate::client::cloud::CapellaClient;
pub use crate::client::cloud::CAPELLA_SRV_SUFFIX;
pub use crate::client::cloud::CLOUD_URL;
pub use crate::client::error::ClientError;
pub use crate::client::http_client::{
    AnalyticsQueryRequest, Endpoint, HTTPClient, ManagementRequest, QueryRequest,
    QueryTransactionRequest, TextSearchQueryRequest, VectorSearchQueryRequest,
};
pub use crate::client::http_handler::HttpResponse;
pub use crate::client::kv_client::{KeyValueRequest, KvClient, KvResponse};
pub use crate::client::tls::RustTlsConfig;
use log::debug;

use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::time::Instant;

extern crate utilities;

mod bedrock_client;
mod capella_ca;
pub(crate) mod cloud;
pub mod cloud_json;
mod codec;
mod crc;
mod error;
mod gemini_client;
pub(crate) mod http_client;
mod http_handler;
mod kv;
mod kv_client;
mod llm_client;
mod openai_client;
mod protocol;
mod tls;

pub use llm_client::LLMClients;

pub struct Client {
    seeds: Vec<String>,
    username: String,
    password: String,
    tls_config: Option<RustTlsConfig>,
}

impl Client {
    pub fn new(
        seeds: Vec<String>,
        username: String,
        password: String,
        tls_config: Option<RustTlsConfig>,
    ) -> Self {
        let seeds = if Client::might_be_srv(&seeds) {
            match utilities::try_lookup_srv(seeds[0].clone()) {
                Ok(s) => s,
                Err(e) => {
                    debug!("Server lookup failed, falling back to hostnames: {}", e);
                    seeds
                }
            }
        } else {
            seeds
        };

        Self {
            seeds,
            username,
            password,
            tls_config,
        }
    }

    pub fn http_client(&self) -> HTTPClient {
        HTTPClient::new(
            self.seeds.clone(),
            self.username.clone(),
            self.password.clone(),
            self.tls_config.clone(),
        )
    }

    pub async fn key_value_client(
        &self,
        bucket: String,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<KvClient, ClientError> {
        KvClient::connect(
            self.seeds.clone(),
            self.username.clone(),
            self.password.clone(),
            self.tls_config.clone(),
            bucket.clone(),
            deadline,
            ctrl_c,
        )
        .await
    }

    // This broadly mirrors the srv logic from the connstr package within gocbcore.
    fn might_be_srv(seeds: &[String]) -> bool {
        if seeds.len() > 1 {
            return false;
        }

        match &seeds[0].parse::<SocketAddr>() {
            Ok(s) => {
                if s.port() > 0 {
                    debug!(
                        "Was able to parse {} to {}, has port so not srv record",
                        &seeds[0], s
                    );
                    return false;
                }
                debug!("Was able to parse {} to {} but no port", &seeds[0], s);
            }
            Err(_) => {
                debug!("Was not able to parse {}", &seeds[0]);
            }
        };

        if Client::is_ip_address(&seeds[0]) {
            return false;
        }

        true
    }

    fn is_ip_address(addr: &String) -> bool {
        match addr.parse::<Ipv6Addr>() {
            Ok(_) => {
                debug!("Address {} is an ip v6 address", &addr);
                return true;
            }
            Err(_) => {
                debug!("Address {} is not an ip v6 address", &addr);
            }
        };
        match addr.parse::<Ipv4Addr>() {
            Ok(_) => {
                debug!("Address {} is an ip v4 address", &addr);
                true
            }
            Err(_) => {
                debug!("Address {} is not an ip v4 address", &addr);
                false
            }
        }
    }
}
