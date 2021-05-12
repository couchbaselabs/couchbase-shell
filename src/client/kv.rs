use crate::client::codec::KeyValueCodec;
use crate::client::protocol::{request, KvRequest, KvResponse, Status};
use crate::client::{protocol, ClientError};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use futures::{SinkExt, StreamExt};
use serde_derive::Deserialize;
use std::collections::{HashMap, HashSet};
use std::convert::TryFrom;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot};
use tokio_util::codec::{FramedRead, FramedWrite};

pub struct KvEndpoint {
    remote_addr: SocketAddr,
    tx: mpsc::Sender<Bytes>,
    opaque: AtomicU32,
    in_flight: Arc<Mutex<HashMap<u32, oneshot::Sender<KvResponse>>>>,
    collections_enabled: bool,
}

impl KvEndpoint {
    pub async fn connect(
        hostname: String,
        port: u32,
        username: String,
        password: String,
        bucket: String,
    ) -> KvEndpoint {
        let remote_addr = format!("{}:{}", hostname, port).parse().unwrap();

        let socket = TcpStream::connect(remote_addr).await.unwrap();

        let (tx, mut rx) = mpsc::channel::<Bytes>(1024);
        let in_flight = Arc::new(Mutex::new(
            HashMap::<u32, oneshot::Sender<KvResponse>>::new(),
        ));
        let mut ep = KvEndpoint {
            remote_addr,
            opaque: AtomicU32::new(0),
            in_flight: Arc::clone(&in_flight),
            tx,
            collections_enabled: false,
        };

        let (r, w) = socket.into_split();
        let mut output = FramedWrite::new(w, KeyValueCodec::new());
        let mut input = FramedRead::new(r, KeyValueCodec::new());

        tokio::spawn(async move {
            loop {
                if let Some(frame) = input.next().await {
                    match frame {
                        Ok(input) => {
                            let response = KvResponse::from(&input.freeze());
                            let requests = Arc::clone(&in_flight);
                            let mut map = requests.lock().unwrap();
                            let t = map.remove(&response.opaque());

                            if let Some(sender) = t {
                                sender.send(response).unwrap();
                            } else {
                                dbg!("it went wrong!");
                            }
                        }
                        Err(_e) => {
                            // For now let's just bail.
                            return;
                        }
                    };
                }
            }
        });

        tokio::spawn(async move {
            loop {
                if let Some(packet) = rx.recv().await {
                    output.send(packet).await.unwrap();
                } else {
                    return;
                }
            }
        });

        let hello_rcvr = ep.send_hello().await.unwrap();
        let err_map_rcvr = ep.send_error_map().await.unwrap();
        let auth_rcvr = ep.send_auth(username, password).await.unwrap();
        let bucket_rcvr = ep.send_select_bucket(bucket).await.unwrap();

        let features = hello_rcvr.await.unwrap().unwrap();
        let error_map = err_map_rcvr.await.unwrap().unwrap();
        auth_rcvr.await.unwrap().unwrap();
        bucket_rcvr.await.unwrap().unwrap();

        if features.contains(&ServerFeature::Collections) {
            ep.collections_enabled = true;
        }

        // println!("Negotiated features {:?}", features);
        // println!("Error Map: {:?}", error_map);
        ep
    }

    fn status_to_error(&self, status: Status) -> Result<(), ClientError> {
        match status {
            Status::Success => Ok(()),
            Status::AuthError => Err(ClientError::AuthError),
            Status::AccessError => Err(ClientError::AccessError),
            Status::KeyNotFound => Err(ClientError::KeyNotFound),
            Status::KeyExists => Err(ClientError::KeyAlreadyExists),
            Status::CollectionUnknown => Err(ClientError::CollectionNotFound),
            Status::ScopeUnknown => Err(ClientError::ScopeNotFound),
            _ => Err(ClientError::RequestFailed {
                reason: Some(status.as_string()),
            }),
        }
    }

    pub async fn get(
        &mut self,
        key: String,
        partition: u16,
        collection_id: u32,
    ) -> Result<KvResponse, ClientError> {
        let req = KvRequest::new(
            protocol::Opcode::Get,
            0,
            partition,
            0,
            Some(key.into()),
            None,
            None,
            collection_id,
        );

        let (tx, rx) = oneshot::channel::<KvResponse>();
        self.send(req, tx).await?;

        let response = rx.await.unwrap();
        self.status_to_error(response.status())?;
        Ok(response)
    }

    async fn send(
        &mut self,
        mut req: KvRequest,
        chan: oneshot::Sender<KvResponse>,
    ) -> Result<(), ClientError> {
        let opaque = self.opaque.fetch_add(1, Ordering::SeqCst);
        req.set_opaque(opaque.clone());
        match self
            .tx
            .send(request(req, self.collections_enabled.clone()).freeze())
            .await
        {
            Ok(_) => {
                let mut map = self.in_flight.lock().unwrap();
                map.insert(opaque, chan);
                Ok(())
            }
            Err(e) => Ok(()),
        }
    }

    async fn send_hello(
        &mut self,
    ) -> Result<oneshot::Receiver<Result<Vec<ServerFeature>, ClientError>>, ClientError> {
        let features = vec![
            ServerFeature::SelectBucket,
            ServerFeature::Xattr,
            ServerFeature::Xerror,
            ServerFeature::AltRequest,
            ServerFeature::SyncReplication,
            ServerFeature::Collections,
            ServerFeature::Tracing,
            ServerFeature::UnorderedExecution,
        ];
        let mut body = BytesMut::with_capacity(features.len() * 2);
        for feature in &features {
            body.put_u16(feature.encoded());
        }

        let req = KvRequest::new(
            protocol::Opcode::Hello,
            0,
            0,
            0,
            None,
            None,
            Some(body.freeze()),
            0,
        );
        let (tx, rx) = oneshot::channel::<KvResponse>();
        self.send(req, tx).await?;

        let (completetx, completerx) =
            oneshot::channel::<Result<Vec<ServerFeature>, ClientError>>();
        tokio::spawn(async move {
            receive_hello(rx, completetx).await;
        });

        Ok(completerx)
    }

    async fn send_error_map(
        &mut self,
    ) -> Result<oneshot::Receiver<Result<ErrorMap, ClientError>>, ClientError> {
        let mut body = BytesMut::with_capacity(2);
        body.put_u16(protocol::ERROR_MAP_VERSION);

        let req = KvRequest::new(
            protocol::Opcode::ErrorMap,
            0,
            0,
            0,
            None,
            None,
            Some(body.freeze()),
            0,
        );
        let (tx, rx) = oneshot::channel::<KvResponse>();
        self.send(req, tx).await?;

        let (completetx, completerx) = oneshot::channel::<Result<ErrorMap, ClientError>>();
        tokio::spawn(async move {
            receive_error_map(rx, completetx).await;
        });

        Ok(completerx)
    }

    async fn send_auth(
        &mut self,
        username: String,
        password: String,
    ) -> Result<oneshot::Receiver<Result<(), ClientError>>, ClientError> {
        let mut body = BytesMut::with_capacity(username.len() + password.len() + 2);
        body.put_u8(0);
        body.put(username.as_bytes());
        body.put_u8(0);
        body.put(password.as_bytes());

        let req = KvRequest::new(
            protocol::Opcode::Auth,
            0,
            0,
            0,
            Some("PLAIN".into()),
            None,
            Some(body.freeze()),
            0,
        );
        let (tx, rx) = oneshot::channel::<KvResponse>();
        self.send(req, tx).await?;

        let (completetx, completerx) = oneshot::channel::<Result<(), ClientError>>();
        tokio::spawn(async move {
            receive_auth(rx, completetx).await;
        });

        Ok(completerx)
    }

    async fn send_select_bucket(
        &mut self,
        bucket: String,
    ) -> Result<oneshot::Receiver<Result<(), ClientError>>, ClientError> {
        let mut key = BytesMut::with_capacity(bucket.len());
        key.put(bucket.as_bytes());

        let req = KvRequest::new(
            protocol::Opcode::SelectBucket,
            0,
            0,
            0,
            Some(key.freeze()),
            None,
            None,
            0,
        );
        let (tx, rx) = oneshot::channel::<KvResponse>();
        self.send(req, tx).await?;

        let (completetx, completerx) = oneshot::channel::<Result<(), ClientError>>();
        tokio::spawn(async move {
            receive_select_bucket(rx, completetx).await;
        });

        Ok(completerx)
    }

    fn close(&mut self) {}
}

async fn receive_hello(
    rx: oneshot::Receiver<KvResponse>,
    completetx: oneshot::Sender<Result<Vec<ServerFeature>, ClientError>>,
) {
    let mut response = rx.await.unwrap();
    let status = response.status();
    let result = match status {
        Status::Success => {
            let mut features = vec![];
            if let Some(mut body) = response.body() {
                let i = 0;
                while body.remaining() > 0 {
                    if let Ok(f) = ServerFeature::try_from(body.get_u16()) {
                        features.push(f);
                    } else {
                        // todo: debug that we got an unknown server feature
                    }
                }
            }

            Ok(features)
        }
        _ => Err(ClientError::RequestFailed {
            reason: Some(status.as_string()),
        }),
    };

    completetx.send(result).unwrap();
}

async fn receive_error_map(
    rx: oneshot::Receiver<KvResponse>,
    completetx: oneshot::Sender<Result<ErrorMap, ClientError>>,
) {
    let mut response = rx.await.unwrap();
    let status = response.status();

    let result = match status {
        Status::Success => {
            if let Some(body) = response.body() {
                let error_map = serde_json::from_slice(body.as_ref()).unwrap();
                Ok(error_map)
            } else {
                Err(ClientError::RequestFailed { reason: None })
            }
        }
        _ => Err(ClientError::RequestFailed {
            reason: Some(status.as_string()),
        }),
    };

    completetx.send(result).unwrap();
}

async fn receive_auth(
    rx: oneshot::Receiver<KvResponse>,
    completetx: oneshot::Sender<Result<(), ClientError>>,
) {
    let mut response = rx.await.unwrap();
    let status = response.status();
    let result = match status {
        Status::Success => Ok(()),
        _ => Err(ClientError::RequestFailed {
            reason: Some(status.as_string()),
        }),
    };

    completetx.send(result).unwrap();
}

async fn receive_select_bucket(
    rx: oneshot::Receiver<KvResponse>,
    completetx: oneshot::Sender<Result<(), ClientError>>,
) {
    let mut response = rx.await.unwrap();
    let status = response.status();
    let result = match status {
        Status::Success => Ok(()),
        _ => Err(ClientError::RequestFailed {
            reason: Some(status.as_string()),
        }),
    };

    completetx.send(result).unwrap();
}

#[derive(Debug)]
enum ServerFeature {
    SelectBucket,
    Xattr,
    Xerror,
    AltRequest,
    SyncReplication,
    Collections,
    Tracing,
    MutationSeqno,
    Snappy,
    UnorderedExecution,
    Vattr,
    CreateAsDeleted,
}

impl ServerFeature {
    pub fn encoded(&self) -> u16 {
        match self {
            Self::SelectBucket => 0x08,
            Self::Xattr => 0x06,
            Self::Xerror => 0x07,
            Self::AltRequest => 0x10,
            Self::SyncReplication => 0x11,
            Self::Collections => 0x12,
            Self::Tracing => 0x0F,
            Self::MutationSeqno => 0x04,
            Self::Snappy => 0x0A,
            Self::UnorderedExecution => 0x0E,
            Self::Vattr => 0x15,
            Self::CreateAsDeleted => 0x17,
        }
    }
}

impl PartialEq for ServerFeature {
    fn eq(&self, other: &Self) -> bool {
        self.encoded() == other.encoded()
    }
}

impl TryFrom<u16> for ServerFeature {
    type Error = u16;

    fn try_from(input: u16) -> Result<Self, Self::Error> {
        Ok(match input {
            0x08 => Self::SelectBucket,
            0x06 => Self::Xattr,
            0x07 => Self::Xerror,
            0x10 => Self::AltRequest,
            0x11 => Self::SyncReplication,
            0x12 => Self::Collections,
            0x0F => Self::Tracing,
            0x04 => Self::MutationSeqno,
            0x0A => Self::Snappy,
            0x0E => Self::UnorderedExecution,
            0x15 => Self::Vattr,
            0x17 => Self::CreateAsDeleted,
            _ => return Err(input),
        })
    }
}

#[derive(Debug, Deserialize)]
struct ErrorMap {
    version: u16,
    revision: u16,
    errors: HashMap<String, ErrorCode>,
}

#[derive(Debug, Deserialize)]
struct ErrorCode {
    name: String,
    desc: String,
    attrs: HashSet<ErrorAttribute>,
    retry: Option<RetrySpecification>,
}

#[derive(Debug, Deserialize)]
struct RetrySpecification {
    strategy: RetryStrategy,
    interval: u32,
    after: u32,
    #[serde(rename = "max-duration")]
    max_duration: u32,
    ceil: u32,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Hash)]
enum ErrorAttribute {
    #[serde(rename = "success")]
    Success,
    #[serde(rename = "item-only")]
    ItemOnly,
    #[serde(rename = "invalid-input")]
    InvalidInput,
    #[serde(rename = "fetch-config")]
    FetchConfig,
    #[serde(rename = "conn-state-invalidated")]
    ConnStateInvalidated,
    #[serde(rename = "auth")]
    Auth,
    #[serde(rename = "special-handling")]
    SpecialHandling,
    #[serde(rename = "support")]
    Support,
    #[serde(rename = "temp")]
    Temp,
    #[serde(rename = "internal")]
    Internal,
    #[serde(rename = "retry-now")]
    RetryNow,
    #[serde(rename = "retry-later")]
    RetryLater,
    #[serde(rename = "subdoc")]
    Subdoc,
    #[serde(rename = "dcp")]
    Dcp,
    #[serde(rename = "auto-retry")]
    AutoRetry,
    #[serde(rename = "item-locked")]
    ItemLocked,
    #[serde(rename = "item-deleted")]
    ItemDeleted,
}

#[derive(Debug, Deserialize)]
enum RetryStrategy {
    #[serde(rename = "exponential")]
    Exponential,
    #[serde(rename = "linear")]
    Linear,
    #[serde(rename = "constant")]
    Constant,
}
