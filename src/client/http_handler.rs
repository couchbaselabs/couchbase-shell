use crate::cli::CtrlcFuture;
use crate::client::error::ClientError;
use crate::config::ClusterTlsConfig;
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

pub(crate) fn status_to_reason(status: u16) -> Option<String> {
    match status {
        400 => Some("bad request".into()),
        401 => Some("unauthorized".into()),
        403 => Some("forbidden".into()),
        404 => Some("not found".into()),
        _ => None,
    }
}

#[derive(Debug)]
pub struct HttpResponse {
    content: String,
    status: u16,
}

impl HttpResponse {
    pub fn new(content: String, status: u16) -> Self {
        Self { content, status }
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn status(&self) -> u16 {
        self.status
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
            return Err(ClientError::Timeout { key: None });
        }
        let timeout = deadline.sub(now);
        let ctrl_c_fut = CtrlcFuture::new(ctrl_c);

        let mut client_builder = ClientBuilder::new();

        if self.tls_config.enabled() {
            if let Some(cert) = self.tls_config.cert_path() {
                let mut buf = Vec::new();
                File::open(cert)
                    .map_err(ClientError::from)?
                    .read_to_end(&mut buf)?;

                client_builder =
                    client_builder.add_root_certificate(reqwest::Certificate::from_pem(&buf)?)
            }

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

        let res_fut = res_builder.send();

        select! {
            result = res_fut => {
                let response = result.map_err(ClientError::from)?;
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
