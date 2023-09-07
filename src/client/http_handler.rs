use crate::cli::CtrlcFuture;
use crate::client::capella_ca::CAPELLA_CERT;
use crate::client::error::ClientError;
use crate::client::Endpoint;
use crate::config::ClusterTlsConfig;
use log::debug;
use reqwest::ClientBuilder;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::ops::Sub;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::{select, time::Instant};

pub enum HttpVerb {
    Delete,
    Get,
    Post,
    Put,
}

impl HttpVerb {
    pub fn as_str(&self) -> &str {
        match self {
            HttpVerb::Get => "GET",
            HttpVerb::Post => "POST",
            HttpVerb::Put => "PUT",
            HttpVerb::Delete => "DELETE",
        }
    }
}

#[derive(Debug)]
pub struct HttpResponse {
    content: String,
    status: u16,
    endpoint: Endpoint,
}

impl HttpResponse {
    pub fn new(content: String, status: u16, endpoint: Endpoint) -> Self {
        Self {
            content,
            status,
            endpoint,
        }
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn status(&self) -> u16 {
        self.status
    }

    pub fn endpoint(&self) -> Endpoint {
        self.endpoint.clone()
    }
}

pub(crate) struct HTTPHandler {
    username: String,
    password: String,
    tls_config: ClusterTlsConfig,
}

impl HTTPHandler {
    pub(crate) fn new(username: String, password: String, tls_config: ClusterTlsConfig) -> Self {
        Self {
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

    pub async fn http_do(
        &self,
        uri: &str,
        method: HttpVerb,
        payload: Option<Vec<u8>>,
        headers: HashMap<&str, &str>,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<(String, u16), ClientError> {
        let uri = format!("{}://{}", self.http_prefix(), uri);
        let now = Instant::now();
        if now >= deadline {
            debug!("HTTP request timed out before sending {}", uri);
            return Err(ClientError::Timeout { key: None });
        }
        let timeout = deadline.sub(now);
        let ctrl_c_fut = CtrlcFuture::new(ctrl_c);

        let mut client_builder = ClientBuilder::new();

        if self.tls_config.enabled() {
            client_builder = if let Some(cert) = self.tls_config.cert_path() {
                let mut buf = Vec::new();
                File::open(cert)
                    .map_err(ClientError::from)?
                    .read_to_end(&mut buf)?;

                client_builder.add_root_certificate(reqwest::Certificate::from_pem(&buf)?)
            } else {
                debug!("Adding Capella root CA to native trust store");
                client_builder
                    .add_root_certificate(reqwest::Certificate::from_pem(CAPELLA_CERT.as_bytes())?)
            };

            if self.tls_config.accept_all_certs() {
                client_builder = client_builder.danger_accept_invalid_certs(true);
            }
        }

        let client = client_builder.build().map_err(ClientError::from)?;
        let mut res_builder = match method {
            HttpVerb::Delete => client.delete(uri),
            HttpVerb::Get => client.get(uri),
            HttpVerb::Post => client.post(uri),
            HttpVerb::Put => client.put(uri),
        };

        res_builder = res_builder
            .basic_auth(&self.username, Some(&self.password))
            .timeout(timeout);

        for (key, value) in headers {
            res_builder = res_builder.header(key, value);
        }

        if let Some(p) = payload {
            res_builder = res_builder.body(p)
        };

        debug!("Performing http request {:?}", &res_builder);

        let res_fut = res_builder.send();

        select! {
            result = res_fut => {
                let response = match result {
                    Ok(r) => Ok(r),
                    Err(e) => {
                        if e.is_timeout() {
                            Err(ClientError::Timeout {
                                key: None,
                            })
                        } else {
                            Err(ClientError::RequestFailed {
                                reason: Some(format!("{}", e)),
                                key: None,
                            })
                        }
                    }
                }?;
                let status = response.status().into();
                let content = response.text().await?;
                Ok((content, status))
            },
            () = ctrl_c_fut => Err(ClientError::Cancelled{key: None}),
        }
    }

    pub(crate) async fn http_get(
        &self,
        uri: &str,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<(String, u16), ClientError> {
        self.http_do(uri, HttpVerb::Get, None, HashMap::new(), deadline, ctrl_c)
            .await
    }

    pub(crate) async fn http_delete(
        &self,
        uri: &str,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<(String, u16), ClientError> {
        self.http_do(
            uri,
            HttpVerb::Delete,
            None,
            HashMap::new(),
            deadline,
            ctrl_c,
        )
        .await
    }

    pub(crate) async fn http_post(
        &self,
        uri: &str,
        payload: Option<Vec<u8>>,
        headers: HashMap<&str, &str>,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<(String, u16), ClientError> {
        self.http_do(uri, HttpVerb::Post, payload, headers, deadline, ctrl_c)
            .await
    }

    pub(crate) async fn http_put(
        &self,
        uri: &str,
        payload: Option<Vec<u8>>,
        headers: HashMap<&str, &str>,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<(String, u16), ClientError> {
        self.http_do(uri, HttpVerb::Put, payload, headers, deadline, ctrl_c)
            .await
    }
}
