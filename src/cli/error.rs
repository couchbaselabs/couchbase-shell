use crate::client::ClientError;
use nu_protocol::{ShellError, Span};
use std::fmt::{Display, Formatter};

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Clone)]
pub enum QueryErrorReason {
    ServiceError,
    AdminError,
    ParseSyntaxError,
    PlanError,
    GeneralError,
    ExecError,
    DatastoreError,
    MultiErrors,
    UnknownError,
}

impl Display for QueryErrorReason {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            QueryErrorReason::ServiceError => "Query service error",
            QueryErrorReason::AdminError => "Query admin error",
            QueryErrorReason::ParseSyntaxError => "Query syntax error",
            QueryErrorReason::PlanError => "Query plan error",
            QueryErrorReason::GeneralError => "Query general error",
            QueryErrorReason::ExecError => "Query exec error",
            QueryErrorReason::DatastoreError => "Query datastore error",
            QueryErrorReason::MultiErrors => "Multiple query errors",
            QueryErrorReason::UnknownError => "Unknown query error",
        };

        write!(f, "{}", message)
    }
}

impl From<i64> for QueryErrorReason {
    fn from(code: i64) -> Self {
        let group = code / 1000;
        match group {
            1 => Self::ServiceError,
            2 => Self::AdminError,
            3 => Self::ParseSyntaxError,
            4 => Self::PlanError,
            5 => Self::GeneralError,
            10..=15 => Self::DatastoreError,
            _ => Self::UnknownError,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Clone)]
pub enum AnalyticsErrorReason {
    AuthorizationError,
    APIError,
    ConnectionError,
    RuntimeError,
    CompilationError,
    InternalError,
    MultiErrors,
    UnknownError,
}

impl Display for AnalyticsErrorReason {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            AnalyticsErrorReason::AuthorizationError => "Analytics authorization error",
            AnalyticsErrorReason::APIError => "Analytics API error",
            AnalyticsErrorReason::ConnectionError => "Analytics connection error",
            AnalyticsErrorReason::RuntimeError => "Analytics runtime error",
            AnalyticsErrorReason::CompilationError => "Analytics compilation error",
            AnalyticsErrorReason::InternalError => "Analytics internal error",
            AnalyticsErrorReason::MultiErrors => "Multiple analytics errors",
            AnalyticsErrorReason::UnknownError => "Unknown analytics error",
        };

        write!(f, "{}", message)
    }
}

impl From<i64> for AnalyticsErrorReason {
    fn from(code: i64) -> Self {
        let group = code / 1000;
        match group {
            20 => Self::AuthorizationError,
            21 => Self::APIError,
            22 => Self::ConnectionError,
            23 => Self::CompilationError,
            24 => Self::CompilationError,
            25 => Self::InternalError,
            _ => Self::UnknownError,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Clone)]
pub enum CBShellError {
    BucketNotFound {
        name: String,
        span: Span,
    },
    ClusterNotFound {
        identifier: String,
        span: Span,
    },
    ClusterNotFoundInConfig {
        name: String,
        span: Span,
    },
    GenericError {
        message: String,
        help: Option<String>,
        span: Option<Span>,
    },
    MalformedResponse {
        message: String,
        response: String,
        span: Span,
    },
    MustBeCapella {
        command_name: String,
        span: Span,
    },
    MustNotBeCapella {
        command_name: String,
        span: Span,
    },
    NoActiveBucket {
        span: Span,
    },
    NoActiveCluster {
        span: Span,
    },
    NoActiveProject {
        span: Option<Span>,
    },
    NoActiveScope {
        span: Span,
    },
    ProjectNotFound {
        name: String,
        span: Span,
    },
    RequestSerializationError {
        message: String,
        span: Span,
    },
    ResponseDeserializationError {
        message: String,
        span: Span,
    },
    UnexpectedResponseStatus {
        status_code: u16,
        message: String,
        span: Span,
    },
    QueryError {
        error_reason: QueryErrorReason,
        status_code: Option<i64>,
        message: String,
        span: Span,
    },
    AnalyticsError {
        error_reason: AnalyticsErrorReason,
        status_code: Option<i64>,
        message: String,
        span: Span,
    },
    OrganizationNotRegistered {
        name: String,
    },
    LLMApiKeyMissing {
        provider: String,
    },
    NoLLMConfigured {},
    EmbedModelMissing {},
    InsufficientColumnarPermissions {
        span: Span,
    },
    ColumnarClustersNotFound {
        project: String,
        span: Span,
    },
}

impl From<CBShellError> for ShellError {
    fn from(ce: CBShellError) -> Self {
        match ce {
            CBShellError::BucketNotFound { name, span } => spanned_shell_error(
                format!("Bucket {} was not found", name),
                "Check that the bucket exists and that permissions are set up correctly".to_string(),
                span,
            ),
            CBShellError::ClusterNotFound {identifier, span} => spanned_shell_error(
                format!("Cluster {} not found in active project", identifier),
                "Check that the cluster you are trying to connect to exists and that it is in the active project".to_string(),
                span
            ),
            CBShellError::ClusterNotFoundInConfig { name, span } => spanned_shell_error(
                format!("Cluster {} was not found in configuration", name),
                "Check configuration file has an entry for the named cluster".to_string(),
                span,
            ),
            CBShellError::GenericError {
                message,
                help,
                span,
            } => spanned_shell_error(message, help, span),
            CBShellError::MalformedResponse {message, response, span} => {
                spanned_shell_error("Malformed response".to_string(), format!("Malformed response, {} - {}. Please raise this as a bug", message, response), span)
            }
            CBShellError::MustBeCapella { command_name, span } => {
                spanned_shell_error(format!("{} can only be used with clusters registered to a Capella organisation", command_name), "Check the configuration file to ensure that the cluster has a capella-organisation entry".to_string(), span)
            },
            CBShellError::MustNotBeCapella { command_name, span } => {
                spanned_shell_error(format!("{} cannot be run against Capella", command_name), "The command cannot be used with Capella clusters.".to_string(), span)
            },
            CBShellError::NoActiveBucket { span } => spanned_shell_error(
                "Unable to determine an active bucket",
                "Set an active bucket using cb-env bucket or by using the --bucket flag if applicable".to_string(),
                span,
            ),
            CBShellError::NoActiveCluster { span } => spanned_shell_error(
                "Unable to determine an active cluster",
                "Set an active cluster using cb-env cluster".to_string(),
                span,
            ),
            CBShellError::NoActiveProject { span } => spanned_shell_error(
                "Unable to determine an active project",
                "Set an active project using cb-env project".to_string(),
                span,
            ),
            CBShellError::NoActiveScope { span } => spanned_shell_error(
                "Unable to determine an active scope",
                "Set an active scope using cb-env scope or by using the --scope flag if applicable".to_string(),
                span,
            ),
            CBShellError::ProjectNotFound { name, span } => spanned_shell_error(
                format!("Project {} was not found", name),
                "Check that the project exists on Capella and that the API key being used has access to it".to_string(),
                span,
            ),
            CBShellError::RequestSerializationError { message, span } => {
                spanned_shell_error("Serialization of the request body failed".to_string(),format!("Error from the serializer: {}", message),span)
            },
            CBShellError::ResponseDeserializationError { message, span } => {
                spanned_shell_error("Deserialization of the request body failed".to_string(),format!("Error from the deserializer: {}", message),span)
            }
            CBShellError::UnexpectedResponseStatus { status_code, message, span} => {
                spanned_shell_error("Unexpected status code".to_string(),format!("Unexpected status code: {}, body: {}", status_code, message), span)
            },
            CBShellError::QueryError {error_reason, status_code, message, span} => {
                let help = match status_code {
                    Some(s) => format!("Received error from query engine, message: {}, code: {}", message, s),
                    None => format!("Received multiple errors from query engine, message: {}", message)
                };
                spanned_shell_error(error_reason.to_string(), help, span)
            },
            CBShellError::AnalyticsError {error_reason, status_code, message, span} => {
                let help = match status_code {
                    Some(s) => format!("Received error from analytics engine, message: {}, code: {}", message, s),
                    None => format!("Received multiple errors from analytics engine, message: {}", message)
                };
                spanned_shell_error(error_reason.to_string(), help, span)
            }
            CBShellError::OrganizationNotRegistered {name} => {
                spanned_shell_error("Organization not registered".to_string(), format!("Has the organization {} been registered in the config file?", name), None)
            }
            CBShellError::LLMApiKeyMissing {provider} => {
                spanned_shell_error(format!("api_key required to use {} models", provider), "Define an api_key in the config/credentials file".to_string(), None)
            }
            CBShellError::NoLLMConfigured {} => {
                spanned_shell_error("no llm defined in config file", "Check the docs at couchbase.sh for examples".to_string(), None)
            }
            CBShellError::EmbedModelMissing {} => {
                spanned_shell_error("no embed model provided", "supply the embed_model in the config file or using the --model flag"
                    .to_string(), None)
            }
            CBShellError::InsufficientColumnarPermissions {span} => {
                spanned_shell_error("Insufficient permissions or the requested object does not exist", "The `sys_data_reader` role must be assigned to your access account to execute queries against the columnar cluster, this can be done through the Capella UI".to_string(), span)
            }
            CBShellError::ColumnarClustersNotFound {project, span} => {
                spanned_shell_error(format!("No columnar clusters found in project {}", project), "You can change the active project with the `cb-env project` command".to_string(), span)
            }
        }
    }
}

fn spanned_shell_error(
    msg: impl Into<String>,
    help: impl Into<Option<String>>,
    span: impl Into<Option<Span>>,
) -> ShellError {
    ShellError::GenericError {
        error: msg.into(),
        msg: "".to_string(),
        span: span.into(),
        help: help.into(),
        inner: Vec::new(),
    }
}

pub fn unexpected_status_code_error(
    status_code: u16,
    message: impl Into<String>,
    span: Span,
) -> ShellError {
    CBShellError::UnexpectedResponseStatus {
        status_code,
        message: message.into(),
        span,
    }
    .into()
}

pub fn no_active_cluster_error(span: Span) -> ShellError {
    CBShellError::NoActiveCluster { span }.into()
}

pub fn no_active_project_error(span: Option<Span>) -> ShellError {
    CBShellError::NoActiveProject { span }.into()
}

pub fn cluster_not_found_error(name: String, span: Span) -> ShellError {
    CBShellError::ClusterNotFoundInConfig { name, span }.into()
}

pub fn no_active_bucket_error(span: Span) -> ShellError {
    CBShellError::NoActiveBucket { span }.into()
}

pub fn bucket_not_found_error(name: String, span: Span) -> ShellError {
    CBShellError::BucketNotFound { name, span }.into()
}

pub fn serialize_error(message: String, span: Span) -> ShellError {
    CBShellError::RequestSerializationError { message, span }.into()
}

pub fn deserialize_error(message: String, span: Span) -> ShellError {
    CBShellError::ResponseDeserializationError { message, span }.into()
}

pub fn organization_not_registered(name: String) -> ShellError {
    CBShellError::OrganizationNotRegistered { name }.into()
}

pub fn llm_api_key_missing(provider: String) -> ShellError {
    CBShellError::LLMApiKeyMissing { provider }.into()
}

pub fn no_llm_configured() -> ShellError {
    CBShellError::NoLLMConfigured {}.into()
}

pub fn embed_model_missing() -> ShellError {
    CBShellError::EmbedModelMissing {}.into()
}

pub fn insufficient_columnar_permissions_error(span: Span) -> ShellError {
    CBShellError::InsufficientColumnarPermissions { span }.into()
}

pub fn columnar_clusters_not_found_error(project: String, span: Span) -> ShellError {
    CBShellError::ColumnarClustersNotFound { project, span }.into()
}

pub fn malformed_response_error(
    message: impl Into<String>,
    response: String,
    span: Span,
) -> ShellError {
    CBShellError::MalformedResponse {
        message: message.into(),
        response,
        span,
    }
    .into()
}

pub fn generic_error(
    message: impl Into<String>,
    help: impl Into<Option<String>>,
    span: impl Into<Option<Span>>,
) -> ShellError {
    CBShellError::GenericError {
        message: message.into(),
        help: help.into(),
        span: span.into(),
    }
    .into()
}

pub fn query_error(
    reason: impl Into<Option<QueryErrorReason>>,
    status_code: impl Into<Option<i64>>,
    message: String,
    span: Span,
) -> ShellError {
    CBShellError::QueryError {
        error_reason: reason.into().unwrap_or(QueryErrorReason::UnknownError),
        status_code: status_code.into(),
        message,
        span,
    }
    .into()
}

pub fn analytics_error(
    reason: impl Into<Option<AnalyticsErrorReason>>,
    status_code: impl Into<Option<i64>>,
    message: String,
    span: Span,
) -> ShellError {
    CBShellError::AnalyticsError {
        error_reason: reason.into().unwrap_or(AnalyticsErrorReason::UnknownError),
        status_code: status_code.into(),
        message,
        span,
    }
    .into()
}

pub fn client_error_to_shell_error(
    error: ClientError,
    span: impl Into<Option<Span>>,
) -> ShellError {
    generic_error(error.message(), error.expanded_message(), span)
}
