pub use crate::client::cloud::{CloudClient, CloudRequest};
pub use crate::client::error::ClientError;
pub use crate::client::http_client::{
    AnalyticsQueryRequest, HTTPClient, ManagementRequest, QueryRequest, SearchQueryRequest,
    ServiceType,
};
pub use crate::client::http_handler::HttpResponse;
pub use crate::client::kv_client::{KeyValueRequest, KvClient};

use crate::config::ClusterTlsConfig;

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

    pub fn key_value_client(&self) -> KvClient {
        KvClient::new(
            self.seeds.clone(),
            self.username.clone(),
            self.password.clone(),
            self.tls_config.clone(),
        )
    }
}
