use crate::client::codec::KeyValueCodec;
use crate::client::protocol::{request, KvRequest, KvResponse, Status};
use crate::client::{protocol, ClientError};
use crate::RustTlsConfig;
use bytes::{Buf, BufMut, Bytes, BytesMut};
use futures::lock::Mutex as AsyncMutex;
use futures::{SinkExt, StreamExt};
use log::{debug, trace, warn};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio::sync::oneshot::Receiver;
use tokio::sync::{mpsc, oneshot};
use tokio_rustls::rustls::pki_types::ServerName;
use tokio_rustls::TlsConnector;
use tokio_util::codec::{FramedRead, FramedWrite};
use uuid::Uuid;

pub struct KvEndpoint {
    tx: mpsc::Sender<Bytes>,
    opaque: AtomicU32,
    in_flight: Arc<AsyncMutex<HashMap<u32, oneshot::Sender<KvResponse>>>>,
    collections_enabled: bool,
    local_addr: String,
    remote_addr: String,
    uuid: String,
    // error_map: Option<ErrorMap>,
}

impl KvEndpoint {
    pub async fn connect(
        hostname: String,
        port: u32,
        username: String,
        password: String,
        bucket: String,
        tls_config: Option<RustTlsConfig>,
    ) -> Result<KvEndpoint, ClientError> {
        let remote_addr = format!("{}:{}", hostname, port);

        debug!(
            "Connecting to {}, TLS enabled: {}",
            &remote_addr,
            tls_config.is_some()
        );

        if let Some(tls_config) = tls_config {
            let tcp_socket = TcpStream::connect(&remote_addr).await.map_err(|e| {
                ClientError::KVCouldNotConnect {
                    reason: e.to_string(),
                    address: remote_addr.clone(),
                }
            })?;
            let local_addr = tcp_socket.local_addr()?;

            let connector = TlsConnector::from(Arc::new(tls_config.config()));
            let socket = connector
                .connect(ServerName::try_from(hostname).unwrap(), tcp_socket)
                .await
                .map_err(|e| ClientError::KVCouldNotConnect {
                    reason: e.to_string(),
                    address: remote_addr.clone(),
                })?;
            KvEndpoint::setup(
                username,
                password,
                bucket,
                socket,
                local_addr.to_string(),
                remote_addr,
            )
            .await
        } else {
            let socket = TcpStream::connect(&remote_addr).await.map_err(|e| {
                ClientError::KVCouldNotConnect {
                    reason: e.to_string(),
                    address: remote_addr.clone(),
                }
            })?;
            let local_addr = socket.local_addr()?;

            KvEndpoint::setup(
                username,
                password,
                bucket,
                socket,
                local_addr.to_string(),
                remote_addr,
            )
            .await
        }
    }

    async fn setup<C: AsyncRead + AsyncWrite + Send + Unpin + 'static>(
        username: String,
        password: String,
        bucket: String,
        stream: C,
        local_addr: String,
        remote_addr: String,
    ) -> Result<KvEndpoint, ClientError> {
        let uuid = Uuid::new_v4().to_string();
        let (tx, mut rx) = mpsc::channel::<Bytes>(1024);
        let in_flight = Arc::new(AsyncMutex::new(
            HashMap::<u32, oneshot::Sender<KvResponse>>::new(),
        ));
        let mut ep = KvEndpoint {
            opaque: AtomicU32::new(0),
            in_flight: Arc::clone(&in_flight),
            tx,
            collections_enabled: false,
            local_addr,
            remote_addr,
            uuid: uuid.clone(),
            // error_map: None,
        };

        let (r, w) = tokio::io::split(stream);
        let mut output = FramedWrite::new(w, KeyValueCodec::new());
        let mut input = FramedRead::new(r, KeyValueCodec::new());

        // Read thread.
        let recv_uuid = uuid.clone();
        tokio::spawn(async move {
            loop {
                if let Some(frame) = input.next().await {
                    match frame {
                        Ok(input) => {
                            let response = KvResponse::from(&input.freeze());
                            trace!(
                                "Resolving response on {}. Opcode={}. Opaque={}. Status={}",
                                recv_uuid,
                                response.opcode(),
                                response.opaque(),
                                response.status(),
                            );
                            let requests = Arc::clone(&in_flight);
                            let mut map = requests.lock().await;
                            let t = map.remove(&response.opaque());
                            drop(map);
                            drop(requests);

                            if let Some(sender) = t {
                                match sender.send(response) {
                                    Ok(_) => {}
                                    Err(_e) => {
                                        warn!("{} could not send kv response", recv_uuid)
                                    }
                                };
                            } else {
                                warn!(
                                    "{} has no entry in request map for {}",
                                    recv_uuid,
                                    &response.opaque()
                                );
                            }
                        }
                        Err(e) => {
                            warn!("{} failed to read frame {}", recv_uuid, e.to_string());
                        }
                    };
                }
            }
        });

        // Send thread.
        let send_uuid = uuid.clone();
        tokio::spawn(async move {
            loop {
                if let Some(packet) = rx.recv().await {
                    match output.send(packet).await {
                        Ok(_) => {}
                        Err(_e) => {
                            warn!("{} could not send kv request", send_uuid);
                        }
                    };
                } else {
                    return;
                }
            }
        });

        let hello_rcvr = ep.send_hello().await?;
        // let err_map_rcvr = ep.send_error_map().await.map(|r| Some(r))?;
        let auth_rcvr = ep.send_auth(username, password).await?;
        let bucket_rcvr = ep.send_select_bucket(bucket).await?;

        let features = match hello_rcvr.await {
            Ok(r) => match r {
                Ok(result) => result,
                Err(e) => {
                    return Err(e);
                }
            },
            Err(e) => {
                return Err(ClientError::RequestFailed {
                    reason: Some(e.to_string()),
                    key: None,
                });
            }
        };
        debug!("{} negotiated features {:?}", ep.uuid, features);
        // if let Some(rcvr) = err_map_rcvr {
        //     let error_map = match rcvr.await {
        //         Ok(r) => match r {
        //             Ok(result) => Some(result),
        //             Err(_e) => None,
        //         },
        //         Err(_e) => None,
        //     };
        //     ep.error_map = error_map;
        // }
        match auth_rcvr.await {
            Ok(r) => match r {
                Ok(result) => result,
                Err(e) => {
                    return Err(e);
                }
            },
            Err(e) => {
                return Err(ClientError::RequestFailed {
                    reason: Some(e.to_string()),
                    key: None,
                });
            }
        };
        debug!("{} authenticated successfully", ep.uuid);
        match bucket_rcvr.await {
            Ok(r) => match r {
                Ok(result) => result,
                Err(e) => {
                    return Err(e);
                }
            },
            Err(e) => {
                return Err(ClientError::RequestFailed {
                    reason: Some(e.to_string()),
                    key: None,
                });
            }
        };

        if features.contains(&ServerFeature::Collections) {
            debug!("{} enabling collections", ep.uuid);
            ep.collections_enabled = true;
        }

        // debug!("Error Map: {:?}", ep.error_map);
        Ok(ep)
    }

    async fn await_response(
        &self,
        rx: Receiver<KvResponse>,
        key: impl Into<Option<String>>,
    ) -> Result<KvResponse, ClientError> {
        match rx.await {
            Ok(r) => Ok(r),
            Err(e) => Err(ClientError::RequestFailed {
                reason: Some(e.to_string()),
                key: key.into(),
            }),
        }
    }

    async fn await_and_handle_doc_response(
        &self,
        rx: Receiver<KvResponse>,
        key: String,
        cid: u32,
        path: impl Into<Option<String>>,
    ) -> Result<KvResponse, ClientError> {
        let mut response = self.await_response(rx, key.clone()).await?;
        let status = response.status();
        if status != Status::Success {
            let reason = ClientError::try_parse_kv_fail_body(&mut response);
            return Err(ClientError::make_kv_doc_op_error(
                status,
                reason,
                key,
                cid,
                path.into(),
            ));
        }
        Ok(response)
    }

    pub async fn get_cid(
        &self,
        scope_name: String,
        collection_name: String,
    ) -> Result<KvResponse, ClientError> {
        let req = KvRequest::new(
            protocol::Opcode::GetCollectionID,
            0,
            0,
            0,
            None,
            None,
            Some(Bytes::from(
                format!("{}.{}", scope_name, collection_name).into_bytes(),
            )),
            0,
        );

        let (tx, rx) = oneshot::channel::<KvResponse>();
        self.send(req, tx).await?;

        let mut resp = self.await_response(rx, None).await?;
        if resp.status() == Status::Success {
            return Ok(resp);
        }
        let reason = ClientError::try_parse_kv_fail_body(&mut resp);
        let error = match resp.status() {
            Status::AuthError => ClientError::AuthError { reason },
            Status::AccessError => ClientError::AccessError { reason },
            Status::CollectionUnknown => ClientError::CollectionNotFound {
                scope_name,
                name: collection_name,
            },
            Status::ScopeUnknown => ClientError::ScopeNotFound { name: scope_name },
            _ => ClientError::RequestFailed {
                reason: Some(reason.unwrap_or_else(|| resp.status().as_string())),
                key: None,
            },
        };
        Err(error)
    }

    pub async fn get(
        &self,
        key: String,
        partition: u16,
        collection_id: u32,
    ) -> Result<KvResponse, ClientError> {
        let req = KvRequest::new(
            protocol::Opcode::Get,
            0,
            partition,
            0,
            Some(Bytes::from(key.clone())),
            None,
            None,
            collection_id,
        );

        let (tx, rx) = oneshot::channel::<KvResponse>();
        self.send(req, tx).await?;

        self.await_and_handle_doc_response(rx, key, collection_id, None)
            .await
    }

    pub async fn sub_doc_get(
        &self,
        key: String,
        partition: u16,
        collection_id: u32,
        path: String,
    ) -> Result<KvResponse, ClientError> {
        let mut extras = BytesMut::with_capacity(4);
        // Extras contain path length and flag
        extras.put_u16(path.len() as u16);
        // 0x0 flag value indicates normal sub doc get
        extras.put_u8(0);

        let req = KvRequest::new(
            protocol::Opcode::SubdocGet,
            0,
            partition,
            0,
            Some(Bytes::from(key.clone())),
            Some(extras.freeze()),
            Some(Bytes::from(path.clone())),
            collection_id,
        );

        let (tx, rx) = oneshot::channel::<KvResponse>();
        self.send(req, tx).await?;

        self.await_and_handle_doc_response(rx, key, collection_id, path)
            .await
    }

    pub async fn sub_doc_multi_lookup(
        &self,
        key: String,
        partition: u16,
        collection_id: u32,
        paths: Vec<String>,
    ) -> Result<KvResponse, ClientError> {
        let mut path_bytes_list: Vec<Vec<u8>> = vec![];
        let mut path_bytes_total = 0;

        for p in paths {
            let path_bytes = Bytes::from(p.clone());
            path_bytes_list.push(path_bytes.to_vec());
            path_bytes_total += path_bytes.len();
        }

        let mut value_buf = BytesMut::with_capacity(path_bytes_total * 4);
        for path_bytes in path_bytes_list {
            value_buf.put_u8(protocol::Opcode::SubdocGet.encoded());
            value_buf.put_u8(0);
            value_buf.put_u16(path_bytes.len() as u16);
            for pb in path_bytes {
                value_buf.put_u8(pb);
            }
        }

        let mut extras = BytesMut::with_capacity(1);
        extras.put_u8(0);

        let req = KvRequest::new(
            protocol::Opcode::SubdocMultiLookup,
            0,
            partition,
            0,
            Some(Bytes::from(key.clone())),
            Some(extras.freeze()),
            Some(value_buf.freeze()),
            collection_id,
        );

        let (tx, rx) = oneshot::channel::<KvResponse>();
        self.send(req, tx).await?;

        self.await_and_handle_doc_response(rx, key, collection_id, None)
            .await
    }

    pub async fn set(
        &self,
        key: String,
        value: Vec<u8>,
        expiry: u32,
        partition: u16,
        collection_id: u32,
    ) -> Result<KvResponse, ClientError> {
        let mut extras = BytesMut::with_capacity(8);
        extras.put_u32(0);
        extras.put_u32(expiry);
        let req = KvRequest::new(
            protocol::Opcode::Set,
            0,
            partition,
            0,
            Some(Bytes::from(key.clone())),
            Some(extras.freeze()),
            Some(value.into()),
            collection_id,
        );

        let (tx, rx) = oneshot::channel::<KvResponse>();
        self.send(req, tx).await?;

        self.await_and_handle_doc_response(rx, key, collection_id, None)
            .await
    }

    pub async fn add(
        &self,
        key: String,
        value: Vec<u8>,
        expiry: u32,
        partition: u16,
        collection_id: u32,
    ) -> Result<KvResponse, ClientError> {
        let mut extras = BytesMut::with_capacity(8);
        extras.put_u32(0);
        extras.put_u32(expiry);
        let req = KvRequest::new(
            protocol::Opcode::Add,
            0,
            partition,
            0,
            Some(Bytes::from(key.clone())),
            Some(extras.freeze()),
            Some(value.into()),
            collection_id,
        );

        let (tx, rx) = oneshot::channel::<KvResponse>();
        self.send(req, tx).await?;

        self.await_and_handle_doc_response(rx, key, collection_id, None)
            .await
    }

    pub async fn replace(
        &self,
        key: String,
        value: Vec<u8>,
        expiry: u32,
        partition: u16,
        collection_id: u32,
    ) -> Result<KvResponse, ClientError> {
        let mut extras = BytesMut::with_capacity(8);
        extras.put_u32(0);
        extras.put_u32(expiry);
        let req = KvRequest::new(
            protocol::Opcode::Replace,
            0,
            partition,
            0,
            Some(Bytes::from(key.clone())),
            Some(extras.freeze()),
            Some(value.into()),
            collection_id,
        );

        let (tx, rx) = oneshot::channel::<KvResponse>();
        self.send(req, tx).await?;

        self.await_and_handle_doc_response(rx, key, collection_id, None)
            .await
    }

    pub async fn remove(
        &self,
        key: String,
        partition: u16,
        collection_id: u32,
    ) -> Result<KvResponse, ClientError> {
        let req = KvRequest::new(
            protocol::Opcode::Remove,
            0,
            partition,
            0,
            Some(Bytes::from(key.clone())),
            None,
            None,
            collection_id,
        );

        let (tx, rx) = oneshot::channel::<KvResponse>();
        self.send(req, tx).await?;

        self.await_and_handle_doc_response(rx, key, collection_id, None)
            .await
    }

    pub async fn noop(&self) -> Result<KvResponse, ClientError> {
        let req = KvRequest::new(protocol::Opcode::Noop, 0, 0, 0, None, None, None, 0);

        let (tx, rx) = oneshot::channel::<KvResponse>();
        self.send(req, tx).await?;

        let mut resp = self.await_response(rx, None).await?;
        if resp.status() == Status::Success {
            return Ok(resp);
        }
        let reason = ClientError::try_parse_kv_fail_body(&mut resp);
        let error = match resp.status() {
            Status::AuthError => ClientError::AuthError { reason },
            Status::AccessError => ClientError::AccessError { reason },
            _ => ClientError::RequestFailed {
                reason: Some(reason.unwrap_or_else(|| resp.status().as_string())),
                key: None,
            },
        };
        Err(error)
    }

    pub fn remote(&self) -> String {
        self.remote_addr.clone()
    }

    async fn send(
        &self,
        mut req: KvRequest,
        chan: oneshot::Sender<KvResponse>,
    ) -> Result<(), ClientError> {
        let opaque = self.opaque.fetch_add(1, Ordering::SeqCst);
        req.set_opaque(opaque);
        trace!(
            "Writing request on {}. {} to {}. Opcode = {}. Opaque = {}",
            self.uuid,
            self.local_addr,
            self.remote_addr,
            req.opcode(),
            req.opaque()
        );
        let mut map = self.in_flight.lock().await;
        map.insert(opaque, chan);
        drop(map);

        match self
            .tx
            .send(request(req, self.collections_enabled).freeze())
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => {
                // If we failed to write the request then immediately drop it.
                let mut map = self.in_flight.lock().await;
                map.remove(&opaque);
                drop(map);

                Err(ClientError::RequestFailed {
                    reason: Some(e.to_string()),
                    key: None,
                })
            }
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

    // async fn send_error_map(
    //     &mut self,
    // ) -> Result<oneshot::Receiver<Result<ErrorMap, ClientError>>, ClientError> {
    //     let mut body = BytesMut::with_capacity(2);
    //     body.put_u16(protocol::ERROR_MAP_VERSION);
    //
    //     let req = KvRequest::new(
    //         protocol::Opcode::ErrorMap,
    //         0,
    //         0,
    //         0,
    //         None,
    //         None,
    //         Some(body.freeze()),
    //         0,
    //     );
    //     let (tx, rx) = oneshot::channel::<KvResponse>();
    //     self.send(req, tx).await?;
    //
    //     let (completetx, completerx) = oneshot::channel::<Result<ErrorMap, ClientError>>();
    //     tokio::spawn(async move {
    //         receive_error_map(rx, completetx).await;
    //     });
    //
    //     Ok(completerx)
    // }

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
            Some(Bytes::from("PLAIN")),
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
}

async fn receive_hello(
    rx: oneshot::Receiver<KvResponse>,
    completetx: oneshot::Sender<Result<Vec<ServerFeature>, ClientError>>,
) {
    let r = match rx.await {
        Ok(r) => Some(r),
        Err(_e) => None,
    };
    let result = if let Some(mut response) = r {
        let status = response.status();
        match status {
            Status::Success => {
                let mut features = vec![];
                if let Some(mut body) = response.body() {
                    while body.remaining() > 0 {
                        if let Ok(f) = ServerFeature::try_from(body.get_u16()) {
                            features.push(f);
                        } else {
                            // todo: debug that we got an unknown server feature
                            warn!(
                                "Server replied with unknown hello feature {:#04x}",
                                body.get_u16()
                            )
                        }
                    }
                }

                Ok(features)
            }
            _ => Err(ClientError::RequestFailed {
                reason: Some(status.as_string()),
                key: None,
            }),
        }
    } else {
        Err(ClientError::RequestFailed {
            reason: None,
            key: None,
        })
    };

    match completetx.send(result) {
        Ok(()) => {}
        Err(_e) => {
            warn!("hello receive failed");
        }
    };
}

// async fn receive_error_map(
//     rx: oneshot::Receiver<KvResponse>,
//     completetx: oneshot::Sender<Result<ErrorMap, ClientError>>,
// ) {
//     let r = match rx.await {
//         Ok(r) => Some(r),
//         Err(_e) => None,
//     };
//     let result = if let Some(mut response) = r {
//         let status = response.status();
//
//         match status {
//             Status::Success => {
//                 if let Some(body) = response.body() {
//                     let error_map = serde_json::from_slice(body.as_ref()).unwrap();
//                     Ok(error_map)
//                 } else {
//                     Err(ClientError::RequestFailed {
//                         reason: None,
//                         key: None,
//                     })
//                 }
//             }
//             _ => Err(ClientError::RequestFailed {
//                 reason: Some(status.as_string()),
//                 key: None,
//             }),
//         }
//     } else {
//         Err(ClientError::RequestFailed {
//             reason: None,
//             key: None,
//         })
//     };
//
//     match completetx.send(result) {
//         Ok(()) => {}
//         Err(_e) => {
//             warn!("error map receive failed");
//         }
//     };
// }

async fn receive_auth(
    rx: oneshot::Receiver<KvResponse>,
    completetx: oneshot::Sender<Result<(), ClientError>>,
) {
    let r = match rx.await {
        Ok(r) => Some(r),
        Err(_e) => None,
    };
    let result = if let Some(response) = r {
        let status = response.status();
        match status {
            Status::Success => Ok(()),
            _ => Err(ClientError::RequestFailed {
                reason: Some(status.as_string()),
                key: None,
            }),
        }
    } else {
        Err(ClientError::RequestFailed {
            reason: None,
            key: None,
        })
    };

    match completetx.send(result) {
        Ok(()) => {}
        Err(_e) => {
            warn!("auth receive failed");
        }
    };
}

async fn receive_select_bucket(
    rx: oneshot::Receiver<KvResponse>,
    completetx: oneshot::Sender<Result<(), ClientError>>,
) {
    let r = match rx.await {
        Ok(r) => Some(r),
        Err(_e) => None,
    };
    let result = if let Some(response) = r {
        let status = response.status();
        match status {
            Status::Success => Ok(()),
            _ => Err(ClientError::RequestFailed {
                reason: Some(status.as_string()),
                key: None,
            }),
        }
    } else {
        Err(ClientError::RequestFailed {
            reason: None,
            key: None,
        })
    };

    match completetx.send(result) {
        Ok(()) => {}
        Err(_e) => {
            warn!("select bucket receive failed");
        }
    };
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

// #[derive(Debug, Deserialize)]
// struct ErrorMap {
//     version: u16,
//     revision: u16,
//     errors: HashMap<String, ErrorCode>,
// }
//
// #[derive(Debug, Deserialize)]
// struct ErrorCode {
//     name: String,
//     desc: String,
//     attrs: HashSet<ErrorAttribute>,
//     retry: Option<RetrySpecification>,
// }
//
// #[derive(Debug, Deserialize)]
// struct RetrySpecification {
//     strategy: RetryStrategy,
//     interval: u32,
//     after: u32,
//     #[serde(rename = "max-duration")]
//     max_duration: u32,
//     ceil: u32,
// }
//
// #[derive(Debug, Deserialize, Eq, PartialEq, Hash)]
// enum ErrorAttribute {
//     #[serde(rename = "success")]
//     Success,
//     #[serde(rename = "item-only")]
//     ItemOnly,
//     #[serde(rename = "invalid-input")]
//     InvalidInput,
//     #[serde(rename = "fetch-config")]
//     FetchConfig,
//     #[serde(rename = "conn-state-invalidated")]
//     ConnStateInvalidated,
//     #[serde(rename = "auth")]
//     Auth,
//     #[serde(rename = "special-handling")]
//     SpecialHandling,
//     #[serde(rename = "support")]
//     Support,
//     #[serde(rename = "temp")]
//     Temp,
//     #[serde(rename = "internal")]
//     Internal,
//     #[serde(rename = "retry-now")]
//     RetryNow,
//     #[serde(rename = "retry-later")]
//     RetryLater,
//     #[serde(rename = "subdoc")]
//     Subdoc,
//     #[serde(rename = "dcp")]
//     Dcp,
//     #[serde(rename = "auto-retry")]
//     AutoRetry,
//     #[serde(rename = "item-locked")]
//     ItemLocked,
//     #[serde(rename = "item-deleted")]
//     ItemDeleted,
// }
//
// #[derive(Debug, Deserialize)]
// enum RetryStrategy {
//     #[serde(rename = "exponential")]
//     Exponential,
//     #[serde(rename = "linear")]
//     Linear,
//     #[serde(rename = "constant")]
//     Constant,
// }
