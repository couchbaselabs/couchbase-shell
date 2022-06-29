pub use crate::client::cloud::{CapellaClient, CapellaRequest};
pub use crate::client::error::ClientError;
pub use crate::client::http_client::{
    AnalyticsQueryRequest, HTTPClient, ManagementRequest, QueryRequest, SearchQueryRequest,
    ServiceType,
};
pub use crate::client::http_handler::HttpResponse;
pub use crate::client::kv_client::{KeyValueRequest, KvClient, KvResponse};
use log::debug;
use nu_protocol::{ShellError, Span};
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::time::Instant;

use crate::config::ClusterTlsConfig;
use trust_dns_resolver::config::{ResolverConfig, ResolverOpts};
use trust_dns_resolver::Resolver;

mod capella_ca;
mod cloud;
mod codec;
mod error;
mod http_client;
mod http_handler;
mod kv;
mod kv_client;
mod protocol;

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
        let seeds = if Client::might_be_srv(&seeds) {
            Client::try_lookup_srv(seeds[0].clone()).unwrap_or(seeds)
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
        span: Span,
    ) -> Result<KvClient, ShellError> {
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
        .map_err(|e| {
            ShellError::GenericError(
                format!("Failed to connect to cluster: {}", e),
                format!(
                    "Check server ports and cluster encryption setting. Does the bucket {} exist?",
                    bucket
                ),
                Some(span),
                None,
                Vec::new(),
            )
        })
    }

    fn try_lookup_srv(addr: String) -> Result<Vec<String>, ClientError> {
        // NOTE: resolver is going to build its own runtime, which is a pain...
        let resolver =
            Resolver::new(ResolverConfig::default(), ResolverOpts::default()).map_err(|e| {
                ClientError::RequestFailed {
                    reason: Some(e.to_string()),
                    key: None,
                }
            })?;
        let mut address = addr;
        if !address.starts_with("_couchbases._tcp.") {
            address = format!("_couchbases._tcp.{}", address);
        }

        let response = match resolver.srv_lookup(address) {
            Ok(k) => k,
            Err(e) => {
                return Err(ClientError::RequestFailed {
                    reason: Some(e.to_string()),
                    key: None,
                })
            }
        };

        let mut addresses: Vec<String> = Vec::new();
        for a in response.iter() {
            // The addresses get suffixed with a . so we have to remove this to later match the address
            // with the addresses in the alternate addresses in the config.
            let mut host = a.target().to_string();
            if let Some(prefix) = host.strip_suffix(".") {
                host = prefix.to_string();
            }
            addresses.push(host);
        }

        Ok(addresses)
    }

    // This broadly mirrors the srv logic from the connstr package within gocbcore.
    fn might_be_srv(seeds: &Vec<String>) -> bool {
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

        return true;
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
        return match addr.parse::<Ipv4Addr>() {
            Ok(_) => {
                debug!("Address {} is an ip v4 address", &addr);
                true
            }
            Err(_) => {
                debug!("Address {} is not an ip v4 address", &addr);
                false
            }
        };
    }
}
