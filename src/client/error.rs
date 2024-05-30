use crate::client::protocol::{KvResponse, Status};
use serde::Deserialize;
use std::fmt;
use std::fmt::Debug;

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Clone, Hash)]
pub enum ConfigurationLoadFailedReason {
    NotFound { bucket: Option<String> },
    Unauthorized,
    Forbidden,
    Unknown { reason: String },
}

impl fmt::Display for ConfigurationLoadFailedReason {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let message = match self {
            ConfigurationLoadFailedReason::NotFound { bucket } => match bucket {
                Some(b) => format!(
                    "Does the bucket {} exist and does the user have permission to access it?",
                    b
                ),
                None => "Could not fetch cluster level config, the endpoint could not be found"
                    .to_string(),
            },
            ConfigurationLoadFailedReason::Unauthorized => {
                "Unauthorized, does the user exist?".to_string()
            }
            ConfigurationLoadFailedReason::Forbidden => {
                "Forbidden, does the user have the correct permissions?".to_string()
            }
            ConfigurationLoadFailedReason::Unknown { reason } => reason.to_string(),
        };

        write!(f, "{}", message)
    }
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Clone, Hash)]
pub enum ClientError {
    ConfigurationLoadFailed {
        reason: ConfigurationLoadFailedReason,
    },
    CollectionNotFound {
        name: String,
        scope_name: String,
    },
    ClusterNotContactable {
        cluster: String,
        reason: String,
    },
    CollectionUnknownDuringRequest {
        key: String,
        cid: u32,
    },
    ScopeNotFound {
        name: String,
    },
    KeyNotFound {
        key: String,
    },
    KeyAlreadyExists {
        key: String,
    },
    AccessError {
        reason: Option<String>,
    },
    AuthError {
        reason: Option<String>,
    },
    Timeout {
        key: Option<String>,
    },
    Cancelled {
        key: Option<String>,
    },
    CapellaClusterNotFound {
        name: String,
    },
    RequestFailed {
        reason: Option<String>,
        key: Option<String>,
    },
    KVCouldNotConnect {
        reason: String,
        address: String,
    },
    PathNotFound {
        key: String,
        path: String,
    },
}

impl ClientError {
    pub fn key(&self) -> Option<String> {
        match self {
            ClientError::CollectionUnknownDuringRequest { key, .. } => Some(key.clone()),
            ClientError::KeyNotFound { key } => Some(key.clone()),
            ClientError::KeyAlreadyExists { key } => Some(key.clone()),
            ClientError::Timeout { key, .. } => key.clone(),
            ClientError::Cancelled { key } => key.clone(),
            ClientError::RequestFailed { key, .. } => key.clone(),
            ClientError::PathNotFound { key, .. } => Some(key.clone()),
            _ => None,
        }
    }

    pub fn message(&self) -> String {
        match self {
            Self::ConfigurationLoadFailed { reason } => {
                let msg = "Failed to load cluster config".to_string();
                match reason {
                    ConfigurationLoadFailedReason::NotFound { bucket } => {
                        if let Some(b) = bucket {
                            format!("{}: bucket '{}' could not be found", msg, b)
                        } else {
                            format!("{}: endpoint could not be found", msg)
                        }
                    }
                    ConfigurationLoadFailedReason::Unauthorized {} => {
                        format!("{}: unauthorized or non-existent user", msg)
                    }
                    ConfigurationLoadFailedReason::Forbidden {} => {
                        format!("{}: user does not have correct permissions", msg)
                    }
                    ConfigurationLoadFailedReason::Unknown { reason } => {
                        format!("{}: {}", msg, reason.to_string())
                    }
                }
            }
            Self::CollectionNotFound { .. } => "Collection unknown".to_string(),
            Self::CollectionUnknownDuringRequest { .. } => {
                "Collection unknown during request".to_string()
            }
            Self::ClusterNotContactable { .. } => "Cluster not contactable".to_string(),
            Self::ScopeNotFound { .. } => "Scope unknown".to_string(),
            Self::KeyNotFound { .. } => "Key not found".to_string(),
            Self::KeyAlreadyExists { .. } => "Key already exists".to_string(),
            Self::AccessError { .. } => "Access error".to_string(),
            Self::AuthError { .. } => "Authentication error".to_string(),
            Self::Timeout { .. } => "Timeout".to_string(),
            Self::Cancelled { .. } => "Request cancelled".to_string(),
            Self::CapellaClusterNotFound { .. } => "Cluster not found".to_string(),
            Self::RequestFailed { reason, .. } => {
                let msg = "Request failed";
                if let Some(r) = reason {
                    format!("{}: {}", msg, r)
                } else {
                    format!("{}: reason unknown", msg)
                }
            }
            Self::KVCouldNotConnect { .. } => "Could not establish kv connection".to_string(),
            Self::PathNotFound { .. } => "Path not found".to_string(),
        }
    }

    pub fn expanded_message(&self) -> String {
        match self {
            Self::ConfigurationLoadFailed { reason } => {
                reason.to_string()
            }
            Self::CollectionNotFound {  name, scope_name, ..} => {
                format!("Collection {}.{} unknown, do both the scope and collection exist?", scope_name, name)
            },
            Self::CollectionUnknownDuringRequest { key, cid } => {
                format!("Collection with ID {} unknown during request for key {}. Were the collection or scope deleted?", cid, key)
            },
            Self::ClusterNotContactable { cluster, reason } => format!(
                "Cluster ({}) not contactable ({}) - check server ports and cluster encryption setting.",
                cluster,
                reason
            ),
            Self::ScopeNotFound { name,.. } => {
                format!("Scope {} unknown, does the scope exist?", name)
            },
            Self::KeyNotFound { key } => format!("Key {} was not found, does it exist in the specified collection?", key),
            Self::KeyAlreadyExists { key } => format!("Key {} already exists, is the correct collection being used?", key),
            Self::AccessError { reason } => {
                if let Some(r) = reason {
                    r.to_string()
                } else {
                    "access error".to_string()
                }
            }
            Self::AuthError { reason } => {
                if let Some(r) = reason {
                    r.to_string()
                } else {
                    "authentication error".to_string()
                }
            }
            Self::Timeout { key } => match key {
                Some(k) => format!("Timeout was observed for key {}, check network latency. Does the timeout need to be longer? You can change it with cb-env timeouts", k),
                None =>  format!("Timeout was observed, check network latency. Does the timeout need to be longer? You can change it with cb-env timeout")
            }
            Self::Cancelled { key } => match key {
                Some(k) => format!("Request was cancelled for {}", k),
                None =>  "Request was cancelled".to_string(),
            }
            Self::CapellaClusterNotFound { name } => format!("Cluster {} was not found within the Capella organisation", name),
            Self::RequestFailed { reason, .. } => match reason.as_ref() {
                Some(re) => re.to_string(),
                None => "Request failed for an unspecified reason".to_string(),
            },
            Self::KVCouldNotConnect { reason, address } => {
                format!("could not connect to {}: {}", address, reason)
            }
            Self::PathNotFound { key, path } => {
                format!("Path {} was not found in doc with key {}", path, key).to_string()
            }
        }
    }

    pub fn try_parse_kv_fail_body(response: &mut KvResponse) -> Option<String> {
        match response.body() {
            Some(b) => match serde_json::from_slice::<KVErrorContext>(&*b) {
                Ok(context) => context.context,
                Err(_) => None,
            },
            None => None,
        }
    }

    pub fn make_kv_doc_op_error(
        status: Status,
        reason: Option<String>,
        key: String,
        cid: u32,
        path: Option<String>,
    ) -> Self {
        match status {
            Status::AuthError => ClientError::AuthError { reason },
            Status::AccessError => ClientError::AccessError { reason },
            Status::KeyNotFound => ClientError::KeyNotFound { key },
            Status::KeyExists => ClientError::KeyAlreadyExists { key },
            Status::PathNotFound => ClientError::PathNotFound {
                key,
                path: path.unwrap_or("".to_string()),
            },
            Status::CollectionUnknown => ClientError::CollectionUnknownDuringRequest { key, cid },
            _ => ClientError::RequestFailed {
                reason: Some(status.as_string()),
                key: Some(key),
            },
        }
    }
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let message = self.message();
        write!(f, "{}", message)
    }
}

impl From<std::io::Error> for ClientError {
    fn from(e: std::io::Error) -> Self {
        ClientError::RequestFailed {
            reason: Some(format!("{}", e)),
            key: None,
        }
    }
}

impl From<serde_json::Error> for ClientError {
    fn from(e: serde_json::Error) -> Self {
        ClientError::RequestFailed {
            reason: Some(format!("{}", e)),
            key: None,
        }
    }
}

impl From<reqwest::Error> for ClientError {
    fn from(e: reqwest::Error) -> Self {
        ClientError::RequestFailed {
            reason: Some(format!("{}", e)),
            key: None,
        }
    }
}

impl From<tokio::sync::oneshot::error::RecvError> for ClientError {
    fn from(e: tokio::sync::oneshot::error::RecvError) -> Self {
        ClientError::RequestFailed {
            reason: Some(e.to_string()),
            key: None,
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct KVErrorContext {
    pub context: Option<String>,
}
