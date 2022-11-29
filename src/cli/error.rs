use crate::client::ClientError;
use nu_protocol::{ShellError, Span};

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Clone)]
pub enum CBShellError {
    BucketNotFound {
        name: String,
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
        span: Span,
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
}

impl From<CBShellError> for ShellError {
    fn from(ce: CBShellError) -> Self {
        match ce {
            CBShellError::BucketNotFound { name, span } => spanned_shell_error(
                format!("Bucket {} was not found", name),
                "Check that the bucket exists and that permissions are set up correctly".to_string(),
                span,
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
                spanned_shell_error(format!("Malformed response, {} - {}", message, response), "Please raise this as a bug".to_string(), span)
            }
            CBShellError::MustBeCapella { command_name, span } => {
                spanned_shell_error(format!("{} can only be used with clusters registered to a Capella organisation", command_name), "Check the configuration file to ensure that the cluster has a capella-organisation entry".to_string(), span)
            },
            CBShellError::MustNotBeCapella { command_name, span } => {
                spanned_shell_error(format!("{} cannot be run against Capella", command_name), "The command cannot be used with Capella clusters. If the cluster is not Capella then check the configuration file to ensure that the cluster does not have a capella-organisation entry".to_string(), span)
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
                "Set an active bucket using cb-env project".to_string(),
                span,
            ),
            CBShellError::NoActiveScope { span } => spanned_shell_error(
                "Unable to determine an active scope",
                "Set an active scope using cb-env scope or by using the --scope flag if applicable".to_string(),
                span,
            ),
            CBShellError::ProjectNotFound { name, span } => spanned_shell_error(
                format!("Project {} was not found", name),
                "Check that the project exists on Capella".to_string(),
                span,
            ),
            CBShellError::RequestSerializationError { message, span } => {
                spanned_shell_error(message, "Serialization of the request body failed".to_string(), span)
            },
            CBShellError::ResponseDeserializationError { message, span } => {
                spanned_shell_error(message, "Deserialization of the response body failed".to_string(), span)
            }
            CBShellError::UnexpectedResponseStatus { status_code, message, span} => {
                spanned_shell_error(format!("Unexpected status code: {}, body: {}", status_code, message), None, span)
            }
        }
    }
}

fn spanned_shell_error(
    msg: impl Into<String>,
    help: impl Into<Option<String>>,
    span: impl Into<Option<Span>>,
) -> ShellError {
    ShellError::GenericError(
        msg.into(),
        "".to_string(),
        span.into(),
        help.into(),
        Vec::new(),
    )
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

pub fn no_active_project_error(span: Span) -> ShellError {
    CBShellError::NoActiveProject { span }.into()
}

pub fn no_active_scope_error(span: Span) -> ShellError {
    CBShellError::NoActiveScope { span }.into()
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
    span: Span,
) -> ShellError {
    CBShellError::GenericError {
        message: message.into(),
        help: help.into(),
        span: Some(span),
    }
    .into()
}

pub fn client_error_to_shell_error(error: ClientError, span: Span) -> ShellError {
    generic_error(error.message(), error.expanded_message(), span)
}
