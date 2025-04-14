use crate::cli::CtrlcFuture;
use crate::client::{ClientError, KvResponse, RustTlsConfig};
use couchbase_core::agent::Agent;
use couchbase_core::agentoptions::{AgentOptions, SeedConfig};
use couchbase_core::authenticator::{Authenticator, PasswordAuthenticator};
use couchbase_core::crudoptions::{
    AddOptions, DeleteOptions, GetOptions, LookupInOptions, ReplaceOptions, UpsertOptions,
};
use couchbase_core::memdx::subdoc::{LookupInOp, LookupInOpType};
use nu_protocol::Signals;
use serde_json::json;
use std::ops::Sub;
use std::sync::Arc;
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
        bucket: String,
        deadline: Instant,
        signals: Signals,
    ) -> Result<Self, ClientError> {
        let tls_config = tls_config.map(|config| Arc::new(config.config()));

        let deadline_sleep = Self::make_timeout(deadline)?;
        let ctrlc_fut = CtrlcFuture::new(signals);

        let agent = select! {
            _ = deadline_sleep => {
                return Err(ClientError::Timeout { key: None });
            }
            _ = ctrlc_fut => {
                return Err(ClientError::Cancelled {key: None});
            }
            agent = Agent::new(
                AgentOptions::new(
                    SeedConfig::new().http_addrs(seeds),
                    Authenticator::PasswordAuthenticator(
                        PasswordAuthenticator{username,password}
                    ),
                ).bucket_name(bucket).tls_config(tls_config),
            ) => {
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
