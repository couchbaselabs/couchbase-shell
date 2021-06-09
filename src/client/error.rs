use nu_errors::ShellError;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Clone, Serialize, Deserialize, Hash)]
pub enum ClientError {
    ConfigurationLoadFailed { reason: Option<String> },
    CollectionManifestLoadFailed { reason: Option<String> },
    CollectionNotFound,
    ScopeNotFound,
    KeyNotFound,
    KeyAlreadyExists,
    AccessError,
    AuthError,
    Timeout,
    Cancelled,
    ClusterNotFound { name: String },
    RequestFailed { reason: Option<String> },
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let message = match self {
            Self::ConfigurationLoadFailed { reason } => match reason.as_ref() {
                Some(re) => format!("failed to load config from cluster: {}", re),
                None => "failed to load config from cluster".into(),
            },
            Self::CollectionManifestLoadFailed { reason } => match reason.as_ref() {
                Some(re) => format!("failed to load collection manifest from cluster: {}", re),
                None => "failed to load collection manifest from cluster".into(),
            },
            Self::CollectionNotFound => "collection not found".into(),
            Self::ScopeNotFound => "scope not found".into(),
            Self::KeyNotFound => "key not found".into(),
            Self::KeyAlreadyExists => "key already exists".into(),
            Self::AccessError => "access error".into(),
            Self::AuthError => "authentication error".into(),
            Self::Timeout => "timeout".into(),
            Self::Cancelled => "request cancelled".into(),
            Self::ClusterNotFound { name } => format!("cluster not found: {}", name),
            Self::RequestFailed { reason } => match reason.as_ref() {
                Some(re) => format!("request failed: {}", re),
                None => "request failed".into(),
            },
        };
        write!(f, "{}", message)
    }
}

impl From<ClientError> for ShellError {
    fn from(ce: ClientError) -> Self {
        // todo: this can definitely be improved with more detail and reporting specifics
        ShellError::untagged_runtime_error(ce.to_string())
    }
}

impl From<std::io::Error> for ClientError {
    fn from(e: std::io::Error) -> Self {
        ClientError::RequestFailed {
            reason: Some(format!("{}", e)),
        }
    }
}

impl From<isahc::Error> for ClientError {
    fn from(e: isahc::Error) -> Self {
        ClientError::RequestFailed {
            reason: Some(format!("{}", e)),
        }
    }
}

impl From<serde_json::Error> for ClientError {
    fn from(e: serde_json::Error) -> Self {
        ClientError::RequestFailed {
            reason: Some(format!("{}", e)),
        }
    }
}

impl From<isahc::http::Error> for ClientError {
    fn from(e: isahc::http::Error) -> Self {
        ClientError::RequestFailed {
            reason: Some(format!("{}", e)),
        }
    }
}
