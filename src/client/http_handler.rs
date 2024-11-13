use crate::cli::{client_error_to_shell_error, CtrlcFuture};
use crate::client::error::ClientError;
use crate::client::Endpoint;
use crate::RustTlsConfig;
use bytes::Bytes;
use futures_core::Stream;
use futures_util::stream::StreamExt;
use log::debug;
use nu_protocol::{ShellError, Signals};
use reqwest::ClientBuilder;
use std::collections::HashMap;
use std::ops::Sub;
use std::pin::Pin;
use std::str::from_utf8;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::{select, time::Instant};

pub type ResultStream = Pin<Box<dyn Stream<Item = Result<Bytes, reqwest::Error>> + Send>>;

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

    // pub fn endpoint(&self) -> Endpoint {
    //     self.endpoint.clone()
    // }
}

pub struct HttpStreamResponse {
    stream: ResultStream,
    status: u16,
    endpoint: Endpoint,
    rt: Arc<Runtime>,
}

impl HttpStreamResponse {
    pub fn new(stream: ResultStream, status: u16, endpoint: Endpoint, rt: Arc<Runtime>) -> Self {
        Self {
            stream,
            status,
            endpoint,
            rt,
        }
    }

    pub fn content(self) -> Result<String, ShellError> {
        self.rt
            .clone()
            .block_on(async { read_stream(self.stream).await })
            .map_err(|e| client_error_to_shell_error(e, None))
    }

    pub fn status(&self) -> u16 {
        self.status
    }

    pub fn endpoint(&self) -> Endpoint {
        self.endpoint.clone()
    }

    pub fn stream(self) -> ResultStream {
        self.stream
    }
}

pub(crate) struct HTTPHandler {
    username: String,
    password: String,
    tls_config: Option<RustTlsConfig>,
}

impl HTTPHandler {
    pub(crate) fn new(
        username: String,
        password: String,
        tls_config: Option<RustTlsConfig>,
    ) -> Self {
        Self {
            username,
            password,
            tls_config,
        }
    }

    fn http_prefix(&self) -> &'static str {
        match self.tls_config.is_some() {
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
        signals: Signals,
    ) -> Result<(ResultStream, u16), ClientError> {
        let uri = format!("{}://{}", self.http_prefix(), uri);
        let now = Instant::now();
        if now >= deadline {
            debug!("HTTP request timed out before sending {}", uri);
            return Err(ClientError::Timeout { key: None });
        }
        let timeout = deadline.sub(now);
        let signals_fut = CtrlcFuture::new(signals);

        let mut client_builder = ClientBuilder::new();

        if let Some(tls_config) = &self.tls_config {
            client_builder = client_builder.use_preconfigured_tls(tls_config.config());
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
                let stream = Box::pin(response.bytes_stream());
                Ok((stream, status))
            },
            () = signals_fut => Err(ClientError::Cancelled{key: None}),
        }
    }

    pub(crate) async fn http_get(
        &self,
        uri: &str,
        deadline: Instant,
        signals: Signals,
    ) -> Result<(ResultStream, u16), ClientError> {
        self.http_do(uri, HttpVerb::Get, None, HashMap::new(), deadline, signals)
            .await
    }

    pub(crate) async fn http_delete(
        &self,
        uri: &str,
        deadline: Instant,
        signals: Signals,
    ) -> Result<(ResultStream, u16), ClientError> {
        self.http_do(
            uri,
            HttpVerb::Delete,
            None,
            HashMap::new(),
            deadline,
            signals,
        )
        .await
    }

    pub(crate) async fn http_put(
        &self,
        uri: &str,
        payload: Option<Vec<u8>>,
        headers: HashMap<&str, &str>,
        deadline: Instant,
        signals: Signals,
    ) -> Result<(ResultStream, u16), ClientError> {
        self.http_do(uri, HttpVerb::Put, payload, headers, deadline, signals)
            .await
    }

    pub(crate) async fn http_post(
        &self,
        uri: &str,
        payload: Option<Vec<u8>>,
        headers: HashMap<&str, &str>,
        deadline: Instant,
        signals: Signals,
    ) -> Result<(ResultStream, u16), ClientError> {
        self.http_do(uri, HttpVerb::Post, payload, headers, deadline, signals)
            .await
    }
}

pub async fn read_stream(mut stream: ResultStream) -> Result<String, ClientError> {
    let mut content = vec![];
    while let Some(result) = stream.next().await {
        match result {
            Ok(b) => content.append(&mut b.to_vec()),
            Err(e) => {
                if e.is_timeout() {
                    return Err(ClientError::Timeout { key: None });
                } else {
                    return Err(ClientError::RequestFailed {
                        reason: Some(format!("{}", e)),
                        key: None,
                    });
                }
            }
        }
    }
    Ok(from_utf8(&content).unwrap().to_string())
}
