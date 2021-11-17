use crate::cli::CtrlcFuture;
use crate::client::error::ClientError;
use crate::config::ClusterTlsConfig;
use isahc::auth::{Authentication, Credentials};
use isahc::config::CaCertificate;
use isahc::{config::SslOption, prelude::*, ResponseFuture};
use std::collections::HashMap;
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

pub(crate) fn status_to_reason(status: u16) -> Option<String> {
    match status {
        400 => Some("bad request".into()),
        401 => Some("unauthorized".into()),
        403 => Some("forbidden".into()),
        404 => Some("not found".into()),
        _ => None,
    }
}

pub(crate) fn http_prefix(tls_config: &ClusterTlsConfig) -> &'static str {
    match tls_config.enabled() {
        true => "https",
        false => "http",
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

    pub(crate) async fn http_do(
        &self,
        mut res_builder: http::request::Builder,
        payload: Option<Vec<u8>>,
        headers: HashMap<&str, &str>,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<(String, u16), ClientError> {
        let now = Instant::now();
        if now >= deadline {
            return Err(ClientError::Timeout { key: None });
        }
        let timeout = deadline.sub(now);
        let ctrl_c_fut = CtrlcFuture::new(ctrl_c);

        res_builder = res_builder
            .authentication(Authentication::basic())
            .credentials(Credentials::new(&self.username, &self.password))
            .timeout(timeout);

        if self.tls_config.enabled() {
            if let Some(cert) = self.tls_config.cert_path() {
                res_builder = res_builder.ssl_ca_certificate(CaCertificate::file(cert));
            }
            res_builder = res_builder.ssl_options(self.http_ssl_opts());
        }

        for (key, value) in headers {
            res_builder = res_builder.header(key, value);
        }

        let res_fut: ResponseFuture;
        if let Some(p) = payload {
            res_fut = res_builder.body(p)?.send_async();
        } else {
            res_fut = res_builder.body(())?.send_async();
        }

        select! {
            result = res_fut => {
                let mut response = result.map_err(ClientError::from)?;
                let content = response.text().await?;
                let status = response.status().into();
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
        let res_builder = isahc::Request::get(uri);
        self.http_do(res_builder, None, HashMap::new(), deadline, ctrl_c)
            .await
    }

    pub(crate) async fn http_delete(
        &self,
        uri: &str,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<(String, u16), ClientError> {
        let res_builder = isahc::Request::delete(uri);
        self.http_do(res_builder, None, HashMap::new(), deadline, ctrl_c)
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
        let res_builder = isahc::Request::post(uri);
        self.http_do(res_builder, payload, headers, deadline, ctrl_c)
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
        let res_builder = isahc::Request::put(uri);
        self.http_do(res_builder, payload, headers, deadline, ctrl_c)
            .await
    }

    pub(crate) fn http_ssl_opts(&self) -> SslOption {
        let mut ssl_opts = SslOption::NONE;
        if !self.tls_config.validate_hostnames() {
            ssl_opts |= SslOption::DANGER_ACCEPT_INVALID_HOSTS;
        }
        if self.tls_config.accept_all_certs() {
            ssl_opts |= SslOption::DANGER_ACCEPT_INVALID_CERTS;
        }
        ssl_opts
    }
}
