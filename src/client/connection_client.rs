use crate::cli::CtrlcFuture;
use crate::client::query_metadata::QueryMetaData;
use crate::client::{ClientError, KvResponse, QueryTransactionRequest, RustTlsConfig};
use couchbase_core::agent::Agent;
use couchbase_core::agentoptions::{AgentOptions, SeedConfig};
use couchbase_core::authenticator::{Authenticator, PasswordAuthenticator};
use couchbase_core::crudoptions::{
    AddOptions, DeleteOptions, GetOptions, LookupInOptions, ReplaceOptions, UpsertOptions,
};
use couchbase_core::memdx::subdoc::{LookupInOp, LookupInOpType};
use couchbase_core::querycomponent::QueryResultStream;
use couchbase_core::queryoptions::QueryOptions;
use futures_util::StreamExt;
use nu_protocol::Signals;
use serde_json::json;
use std::collections::HashMap;
use std::ops::Sub;
use std::sync::Arc;
use std::time::Duration;
use tokio::select;
use tokio::time::{sleep, Instant, Sleep};

pub struct ConnectionClient {
    agent: Agent,
}

impl ConnectionClient {
    pub async fn connect(
        seeds: Vec<String>,
        username: String,
        password: String,
        tls_config: Option<RustTlsConfig>,
        bucket: Option<String>,
        deadline: Instant,
        signals: Signals,
    ) -> Result<Self, ClientError> {
        let tls_config = tls_config.map(|config| Arc::new(config.config()));

        let deadline_sleep = Self::make_timeout(deadline)?;
        let ctrlc_fut = CtrlcFuture::new(signals);

        let mut opts = AgentOptions::new(
            SeedConfig::new().http_addrs(seeds),
            Authenticator::PasswordAuthenticator(PasswordAuthenticator { username, password }),
        )
        .tls_config(tls_config);

        if let Some(bucket) = bucket {
            opts = opts.bucket_name(bucket);
        }

        let agent = select! {
            _ = deadline_sleep => {
                return Err(ClientError::Timeout { key: None });
            }
            _ = ctrlc_fut => {
                return Err(ClientError::Cancelled {key: None});
            }
            agent = Agent::new(opts) => {
                agent.map_err(ClientError::from)?
            }
        };

        Ok(Self { agent })
    }

    pub async fn get(
        &self,
        req: GetRequest<'_>,
        deadline: Instant,
        signals: Signals,
    ) -> Result<KvResponse, ClientError> {
        let ctrlc_fut = CtrlcFuture::new(signals);
        let deadline_sleep = Self::make_timeout(deadline)?;

        let result = select! {
            _ = deadline_sleep => {
                return Err(ClientError::Timeout { key: None });
            }
            _ = ctrlc_fut => {
                return Err(ClientError::Cancelled {key: None});
            }
            result = self.agent.get(GetOptions::new(req.key.as_bytes(), &req.scope, &req.collection)) => {
                result.map_err(ClientError::from)?
            }
        };

        let content = serde_json::from_slice(&result.value)?;

        Ok(KvResponse {
            content: Some(content),
            cas: result.cas,
            key: req.key.to_string(),
        })
    }

    pub async fn set(
        &self,
        req: SetRequest<'_>,
        deadline: Instant,
        signals: Signals,
    ) -> Result<KvResponse, ClientError> {
        let ctrlc_fut = CtrlcFuture::new(signals);
        let deadline_sleep = Self::make_timeout(deadline)?;

        let result = select! {
            _ = deadline_sleep => {
                return Err(ClientError::Timeout { key: None });
            }
            _ = ctrlc_fut => {
                return Err(ClientError::Cancelled {key: None});
            }
            result = self.agent.upsert(UpsertOptions::new(req.key.as_bytes(), req.scope, req.collection, req.value).expiry(req.expiry)) => {
                result.map_err(ClientError::from)?
            }
        };

        Ok(KvResponse {
            content: None,
            cas: result.cas,
            key: req.key.to_string(),
        })
    }

    pub async fn insert(
        &self,
        req: InsertRequest<'_>,
        deadline: Instant,
        signals: Signals,
    ) -> Result<KvResponse, ClientError> {
        let ctrlc_fut = CtrlcFuture::new(signals);
        let deadline_sleep = Self::make_timeout(deadline)?;

        let result = select! {
            _ = deadline_sleep => {
                return Err(ClientError::Timeout { key: None });
            }
            _ = ctrlc_fut => {
                return Err(ClientError::Cancelled {key: None});
            }
            result = self.agent.add(AddOptions::new(req.key.as_bytes(), req.scope, req.collection, req.value).expiry(req.expiry)) => {
                result.map_err(ClientError::from)?
            }
        };

        Ok(KvResponse {
            content: None,
            cas: result.cas,
            key: req.key.to_string(),
        })
    }

    pub async fn replace(
        &self,
        req: ReplaceRequest<'_>,
        deadline: Instant,
        signals: Signals,
    ) -> Result<KvResponse, ClientError> {
        let ctrlc_fut = CtrlcFuture::new(signals);
        let deadline_sleep = Self::make_timeout(deadline)?;

        let result = select! {
            _ = deadline_sleep => {
                return Err(ClientError::Timeout { key: None });
            }
            _ = ctrlc_fut => {
                return Err(ClientError::Cancelled {key: None});
            }
            result = self.agent.replace(ReplaceOptions::new(req.key.as_bytes(), req.scope, req.collection, req.value).expiry(req.expiry)) => {
                result.map_err(ClientError::from)?
            }
        };

        Ok(KvResponse {
            content: None,
            cas: result.cas,
            key: req.key.to_string(),
        })
    }

    pub async fn remove(
        &self,
        req: RemoveRequest<'_>,
        deadline: Instant,
        signals: Signals,
    ) -> Result<KvResponse, ClientError> {
        let ctrlc_fut = CtrlcFuture::new(signals);
        let deadline_sleep = Self::make_timeout(deadline)?;

        let result = select! {
            _ = deadline_sleep => {
                return Err(ClientError::Timeout { key: None });
            }
            _ = ctrlc_fut => {
                return Err(ClientError::Cancelled {key: None});
            }
            result = self.agent.delete(DeleteOptions::new(req.key.as_bytes(), &req.scope, &req.collection)) => {
                result.map_err(ClientError::from)?
            }
        };

        Ok(KvResponse {
            content: None,
            cas: result.cas,
            key: req.key.to_string(),
        })
    }

    pub async fn lookup_in(
        &self,
        req: LookupInRequest<'_>,
        deadline: Instant,
        signals: Signals,
    ) -> Result<KvResponse, ClientError> {
        let ctrlc_fut = CtrlcFuture::new(signals);
        let deadline_sleep = Self::make_timeout(deadline)?;

        let mut ops = Vec::new();
        for path in req.paths {
            ops.push(LookupInOp::new(LookupInOpType::Get, path.as_bytes()));
        }

        let result = select! {
            _ = deadline_sleep => {
                return Err(ClientError::Timeout { key: None });
            }
            _ = ctrlc_fut => {
                return Err(ClientError::Cancelled {key: None});
            }
            result = self.agent.lookup_in(LookupInOptions::new(req.key.as_bytes(), &req.scope, &req.collection, &ops)) => {
                result.map_err(ClientError::from)?
            }
        };

        let mut results: Vec<serde_json::Value> = vec![];
        let ops = result.value;

        for op in ops.iter() {
            if let Some(err) = &op.err {
                results.push(serde_json::to_value(&err.to_string())?);
            }
            if let Some(value) = &op.value {
                results.push(serde_json::from_slice(value)?);
            }
        }

        Ok(KvResponse {
            content: Some(json!(results)),
            cas: result.cas,
            key: req.key.to_string(),
        })
    }

    pub async fn query(
        &self,
        req: QueryRequest<'_>,
        deadline: Instant,
        signals: Signals,
    ) -> Result<HttpStreamResponse, ClientError> {
        let ctrlc_fut = CtrlcFuture::new(signals);
        let deadline_sleep = Self::make_timeout(deadline)?;

        let mut opts = QueryOptions::new()
            .statement(Some(req.statement.to_string()))
            .timeout(Some(req.timeout));
        if let Some(params) = req.parameters {
            match params {
                serde_json::Value::Array(params) => {
                    opts = opts.args(params);
                }
                serde_json::Value::Object(map) => {
                    let mut hmap = HashMap::new();
                    for (key, value) in map.into_iter() {
                        hmap.insert(key, value);
                    }

                    opts = opts.named_args(hmap);
                }
                _ => {}
            }
        }
        if let Some(scope) = req.scope {
            opts = opts.query_context(Some(format!("`{}`.`{}`", scope.0, scope.1)));
        }
        if let Some(transaction) = req.transaction {
            let mut raw = HashMap::new();
            if let Some(timeout) = transaction.tx_timeout {
                raw.insert(
                    "timeout".to_string(),
                    serde_json::Value::String(format!("{}ms", timeout.as_millis())),
                );
            }
            if let Some(tx_id) = transaction.tx_id {
                raw.insert("txid".to_string(), serde_json::Value::String(tx_id));
            }
            if let Some(ep) = transaction.endpoint {
                opts = opts.endpoint(Some(ep.to_string()));
            }
            opts = opts.raw(Some(raw));
        }

        let result = select! {
            _ = deadline_sleep => {
                return Err(ClientError::Timeout { key: None });
            }
            _ = ctrlc_fut => {
                return Err(ClientError::Cancelled {key: None});
            }
            result = self.agent.query(opts) => {
                result.map_err(ClientError::from)?
            }
        };

        let ep = result.endpoint().to_string();
        Ok(HttpStreamResponse::new(
            ResultStream::QueryResultStream(result),
            ep,
        ))
    }

    fn make_timeout(deadline: Instant) -> Result<Sleep, ClientError> {
        let now = Instant::now();
        if now >= deadline {
            return Err(ClientError::Timeout { key: None });
        }

        Ok(sleep(deadline.sub(now)))
    }
}

#[derive(Clone, Debug, Copy)]
pub struct GetRequest<'a> {
    pub key: &'a str,
    pub scope: &'a str,
    pub collection: &'a str,
}

#[derive(Clone, Debug, Copy)]
pub struct SetRequest<'a> {
    pub key: &'a str,
    pub value: &'a [u8],
    pub expiry: u32,
    pub scope: &'a str,
    pub collection: &'a str,
}

#[derive(Clone, Debug, Copy)]
pub struct InsertRequest<'a> {
    pub key: &'a str,
    pub value: &'a [u8],
    pub expiry: u32,
    pub scope: &'a str,
    pub collection: &'a str,
}

#[derive(Clone, Debug, Copy)]
pub struct ReplaceRequest<'a> {
    pub key: &'a str,
    pub value: &'a [u8],
    pub expiry: u32,
    pub scope: &'a str,
    pub collection: &'a str,
}

#[derive(Clone, Debug, Copy)]
pub struct RemoveRequest<'a> {
    pub key: &'a str,
    pub scope: &'a str,
    pub collection: &'a str,
}

#[derive(Clone, Debug)]
pub struct LookupInRequest<'a> {
    pub key: &'a str,
    pub paths: Vec<&'a str>,
    pub scope: &'a str,
    pub collection: &'a str,
}

#[derive(Clone, Debug)]
pub struct QueryRequest<'a> {
    pub statement: &'a str,
    pub parameters: Option<serde_json::Value>,
    pub scope: Option<(String, String)>,
    pub timeout: Duration,
    pub transaction: Option<QueryTransactionRequest>,
}

pub(crate) enum ResultStream {
    QueryResultStream(QueryResultStream),
}

impl ResultStream {
    pub async fn next(&mut self) -> Option<Result<Vec<u8>, ClientError>> {
        match self {
            ResultStream::QueryResultStream(ref mut stream) => {
                stream.next().await.map(|n| match n {
                    Ok(result) => Ok(result.to_vec()),
                    Err(e) => Err(ClientError::from(e)),
                })
            }
        }
    }

    pub fn metadata(&self) -> Result<Option<ResponseMetadata>, ClientError> {
        match self {
            ResultStream::QueryResultStream(ref stream) => {
                let metadata: QueryMetaData = stream.metadata()?.clone().into();
                Ok(Some(ResponseMetadata::Query(metadata)))
            }
        }
    }
}

pub enum ResponseMetadata {
    Query(QueryMetaData),
}

impl ResponseMetadata {
    pub fn query(&self) -> Option<&QueryMetaData> {
        match self {
            ResponseMetadata::Query(ref metadata) => Some(metadata),
        }
    }
}

pub struct HttpStreamResponse {
    stream: ResultStream,
    endpoint: String,
}

impl HttpStreamResponse {
    pub(crate) fn new(stream: ResultStream, endpoint: String) -> Self {
        Self { stream, endpoint }
    }

    pub async fn content(&mut self) -> Result<Vec<Vec<u8>>, ClientError> {
        read_stream(&mut self.stream).await
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    // pub fn stream(self) -> ResultStream {
    //     self.stream
    // }

    pub fn metadata(&self) -> Result<Option<ResponseMetadata>, ClientError> {
        self.stream.metadata()
    }
}

pub async fn read_stream(stream: &mut ResultStream) -> Result<Vec<Vec<u8>>, ClientError> {
    let mut content = vec![];

    while let Some(result) = stream.next().await {
        match result {
            Ok(b) => content.push(b.to_vec()),
            Err(e) => {
                return Err(ClientError::RequestFailed {
                    reason: Some(format!("{}", e)),
                    key: None,
                });
            }
        }
    }

    Ok(content)
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct Endpoint {
    hostname: String,
    port: u32,
}
