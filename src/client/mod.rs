pub use crate::client::cloud::{CloudClient, CloudRequest};
pub use crate::client::error::ClientError;
pub use crate::client::http_client::{
    AnalyticsQueryRequest, HTTPClient, ManagementRequest, QueryRequest, SearchQueryRequest,
    ServiceType,
};
pub use crate::client::http_handler::HttpResponse;
pub use crate::client::kv_client::{KeyValueRequest, KvClient};

use crate::config::ClusterTlsConfig;
use trust_dns_resolver::config::{ResolverConfig, ResolverOpts};
use trust_dns_resolver::Resolver;

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

    pub fn http_client_with_seeds(&self, seeds: Vec<String>) -> HTTPClient {
        HTTPClient::new(
            seeds,
            self.username.clone(),
            self.password.clone(),
            self.tls_config.clone(),
        )
    }

    pub fn key_value_client(&self) -> KvClient {
        KvClient::new(
            self.seeds.clone(),
            self.username.clone(),
            self.password.clone(),
            self.tls_config.clone(),
        )
    }

    pub fn key_value_client_with_seeds(&self, seeds: Vec<String>) -> KvClient {
        KvClient::new(
            seeds,
            self.username.clone(),
            self.password.clone(),
            self.tls_config.clone(),
        )
    }

    pub fn try_lookup_srv(addr: String) -> Result<Vec<String>, ClientError> {
        let resolver =
            Resolver::new(ResolverConfig::default(), ResolverOpts::default()).map_err(|e| {
                ClientError::RequestFailed {
                    reason: Some(e.to_string()),
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
}
