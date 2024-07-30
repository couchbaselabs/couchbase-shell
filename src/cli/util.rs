use crate::cli::error::CBShellError::{
    GenericError, MustNotBeCapella, ProjectNotFound, UnexpectedResponseStatus,
};
use crate::cli::error::{
    client_error_to_shell_error, cluster_not_found_error, deserialize_error,
    malformed_response_error, no_active_bucket_error, unexpected_status_code_error,
};
use crate::cli::CBShellError::ClusterNotFound;
use crate::client::cloud_json::{OrganizationsResponse, ProjectsResponse};
use crate::client::{CapellaClient, CapellaRequest, HttpResponse};
use crate::state::State;
use crate::{RemoteCluster, RemoteClusterType};
use nu_engine::CallExt;
use nu_protocol::ast::{Call, PathMember};
use nu_protocol::engine::{EngineState, Stack};
use nu_protocol::Record;
use nu_protocol::{IntoPipelineData, PipelineData, ShellError, Span, Value};
use nu_utils::SharedCow;
use num_traits::cast::ToPrimitive;
use regex::Regex;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;
use tokio::time::Instant;

pub fn is_http_status(response: &HttpResponse, status: u16, span: Span) -> Result<(), ShellError> {
    if response.status() != status {
        return Err(unexpected_status_code_error(
            response.status(),
            response.content(),
            span,
        ));
    }

    Ok(())
}

pub fn convert_row_to_nu_value(
    v: &serde_json::Value,
    span: Span,
    cluster_identifier: String,
) -> Result<Value, ShellError> {
    match v {
        serde_json::Value::Object(o) => {
            let mut cols = vec![];
            let mut vals = vec![];
            for (k, v) in o.iter() {
                cols.push(k.clone());
                vals.push(convert_json_value_to_nu_value(v, span)?);
            }
            cols.push("cluster".to_string());
            vals.push(Value::String {
                val: cluster_identifier,
                internal_span: span,
            });

            Ok(Value::Record {
                val: SharedCow::new(Record::from_raw_cols_vals(cols, vals, span, span).unwrap()),
                internal_span: span,
            })
        }
        _ => Err(malformed_response_error(
            "row was not an object",
            v.to_string(),
            span,
        )),
    }
}

pub fn convert_json_value_to_nu_value(
    v: &serde_json::Value,
    span: Span,
) -> Result<Value, ShellError> {
    let result = match v {
        serde_json::Value::Null => Value::Nothing {
            internal_span: span,
        },
        serde_json::Value::Bool(b) => Value::Bool {
            val: *b,
            internal_span: span,
        },
        serde_json::Value::Number(n) => {
            if let Some(val) = n.as_i64() {
                Value::Int {
                    val,
                    internal_span: span,
                }
            } else if let Some(val) = n.as_f64() {
                Value::Float {
                    val,
                    internal_span: span,
                }
            } else {
                return Err(GenericError {
                    message: format!(
                        "Unexpected numeric value, cannot convert {} into i64 or f64",
                        n
                    ),
                    help: None,
                    span: Some(span),
                }
                .into());
            }
        }
        serde_json::Value::String(val) => Value::String {
            val: val.clone(),
            internal_span: span,
        },
        serde_json::Value::Array(a) => {
            let t = a
                .iter()
                .map(|x| convert_json_value_to_nu_value(x, span))
                .collect::<Result<Vec<Value>, ShellError>>()?;
            Value::List {
                vals: t,
                internal_span: span,
            }
        }
        serde_json::Value::Object(o) => {
            let mut cols = vec![];
            let mut vals = vec![];

            for (k, v) in o.iter() {
                cols.push(k.clone());
                vals.push(convert_json_value_to_nu_value(v, span)?);
            }

            Value::Record {
                val: SharedCow::new(Record::from_raw_cols_vals(cols, vals, span, span).unwrap()),
                internal_span: span,
            }
        }
    };

    Ok(result)
}

// Adapted from https://github.com/nushell/nushell/blob/main/crates/nu-command/src/commands/formats/to/json.rs
pub fn convert_nu_value_to_json_value(
    v: &Value,
    span: Span,
) -> Result<serde_json::Value, ShellError> {
    Ok(match v {
        Value::Bool { val, .. } => serde_json::Value::Bool(*val),
        Value::Filesize { val, .. } => serde_json::Value::Number(serde_json::Number::from(
            val.to_u64().expect("What about really big numbers"),
        )),
        Value::Duration { val, .. } => serde_json::Value::String(val.to_string()),
        Value::Date { val, .. } => serde_json::Value::String(val.to_string()),
        Value::Float { val, .. } => {
            if let Some(num) = serde_json::Number::from_f64(*val) {
                serde_json::Value::Number(num)
            } else {
                return Err(GenericError {
                    message: format!("Unexpected numeric value, cannot convert {} from f64", val),
                    help: None,
                    span: Some(span),
                }
                .into());
            }
        }
        Value::Int { val, .. } => serde_json::Value::Number(serde_json::Number::from(*val)),
        Value::Nothing { .. } => serde_json::Value::Null,
        Value::String { val, .. } => serde_json::Value::String(val.clone()),
        Value::CellPath { val, .. } => serde_json::Value::Array(
            val.members
                .iter()
                .map(|x| match &x {
                    PathMember::String { val, .. } => Ok(serde_json::Value::String(val.clone())),
                    PathMember::Int { val, .. } => Ok(serde_json::Value::Number(
                        serde_json::Number::from(*val as u64),
                    )),
                })
                .collect::<Result<Vec<serde_json::Value>, ShellError>>()?,
        ),
        Value::List { vals, .. } => serde_json::Value::Array(json_list(vals, span)?),
        Value::Error { error, .. } => return Err(*error.clone()),
        Value::Binary { val, .. } => serde_json::Value::Array(
            val.iter()
                .map(|x| {
                    Ok(serde_json::Value::Number(serde_json::Number::from(
                        *x as u64,
                    )))
                })
                .collect::<Result<Vec<serde_json::Value>, ShellError>>()?,
        ),
        Value::Record { val, .. } => {
            let mut m = serde_json::Map::new();
            for (k, v) in val.iter() {
                m.insert(k.clone(), convert_nu_value_to_json_value(v, span)?);
            }
            serde_json::Value::Object(m)
        }
        Value::Custom { .. } => serde_json::Value::Null,
        Value::Range { .. } => serde_json::Value::Null,
        Value::Closure { .. } => serde_json::Value::Null,
        Value::Glob { val, .. } => serde_json::Value::String(val.clone()),
    })
}

fn json_list(input: &[Value], span: Span) -> Result<Vec<serde_json::Value>, ShellError> {
    let mut out = vec![];

    for value in input {
        out.push(convert_nu_value_to_json_value(value, span)?);
    }

    Ok(out)
}

pub fn cluster_identifiers_from(
    engine_state: &EngineState,
    stack: &mut Stack,
    state: &Arc<Mutex<State>>,
    args: &Call,
    default_active: bool,
) -> Result<Vec<String>, ShellError> {
    let state = state.lock().unwrap();
    let identifier_arg: String = match args.get_flag(engine_state, stack, "clusters")? {
        Some(arg) => arg,
        None => {
            if default_active {
                return Ok(vec![state.active()]);
            }
            "".to_string()
        }
    };

    let re = match Regex::new(identifier_arg.as_str()) {
        Ok(v) => v,
        Err(e) => {
            return Err(GenericError {
                message: e.to_string(),
                help: Some("Failed to parse identifier used for specifying clusters".to_string()),
                span: Some(args.head),
            }
            .into());
        }
    };
    let clusters: Vec<String> = state
        .clusters()
        .keys()
        .filter(|k| re.is_match(k))
        .cloned()
        .collect();
    if clusters.is_empty() {
        return Err(cluster_not_found_error(identifier_arg, args.span()));
    }

    Ok(clusters)
}

pub fn namespace_from_args(
    bucket_flag: Option<String>,
    scope_flag: Option<String>,
    collection_flag: Option<String>,
    active_cluster: &RemoteCluster,
    span: Span,
) -> Result<(String, String, String), ShellError> {
    let bucket = match bucket_flag.or_else(|| active_cluster.active_bucket()) {
        Some(v) => Ok(v),
        None => Err(no_active_bucket_error(span)),
    }?;

    let scope = match scope_flag {
        Some(s) => s,
        None => match active_cluster.active_scope() {
            Some(s) => s,
            None => "".into(),
        },
    };

    let collection = match collection_flag {
        Some(c) => c,
        None => match active_cluster.active_collection() {
            Some(c) => c,
            None => "".into(),
        },
    };

    Ok((bucket, scope, collection))
}

pub fn validate_is_not_cloud(
    cluster: &RemoteCluster,
    command_name: impl Into<String>,
    span: Span,
) -> Result<(), ShellError> {
    match cluster.cluster_type() {
        RemoteClusterType::Other => Ok(()),
        _ => Err(MustNotBeCapella {
            command_name: command_name.into(),
            span,
        }
        .into()),
    }
}

// We take a conn_string instead of name since cluster local identfiers can differ from names of
// clusters
pub(crate) fn cluster_id_from_conn_str(
    identifier: String,
    ctrl_c: Arc<AtomicBool>,
    hostnames: Vec<String>,
    client: &Arc<CapellaClient>,
    deadline: Instant,
    span: Span,
    org_id: String,
    project_id: String,
) -> Result<String, ShellError> {
    let response = client
        .get_clusters(org_id, project_id, deadline, ctrl_c)
        .map_err(|e| client_error_to_shell_error(e, span))?;

    for c in response.items() {
        for conn_str in hostnames.clone() {
            if c.connection_string().contains(conn_str.as_str()) {
                return Ok(c.id().to_string());
            }
        }
    }

    Err(ShellError::from(ClusterNotFound { identifier, span }))
}

pub(crate) fn find_project_id(
    ctrl_c: Arc<AtomicBool>,
    name: String,
    client: &Arc<CapellaClient>,
    deadline: Instant,
    span: Span,
    org_id: String,
) -> Result<String, ShellError> {
    let response = client
        .capella_request(CapellaRequest::GetProjects { org_id }, deadline, ctrl_c)
        .map_err(|e| client_error_to_shell_error(e, span))?;
    if response.status() != 200 {
        return Err(UnexpectedResponseStatus {
            status_code: response.status(),
            message: response.content().to_string(),
            span,
        }
        .into());
    }
    let content: ProjectsResponse = serde_json::from_str(response.content())
        .map_err(|e| deserialize_error(e.to_string(), span))?;

    for p in content.items() {
        if p.name() == name.clone() {
            return Ok(p.id().to_string());
        }
    }

    Err(ShellError::from(ProjectNotFound { name, span }))
}

pub(crate) fn find_org_id(
    ctrl_c: Arc<AtomicBool>,
    client: &Arc<CapellaClient>,
    deadline: Instant,
    span: Span,
) -> Result<String, ShellError> {
    let response = client
        .capella_request(CapellaRequest::GetOrganizations {}, deadline, ctrl_c)
        .map_err(|e| client_error_to_shell_error(e, span))?;
    if response.status() != 200 {
        return Err(UnexpectedResponseStatus {
            status_code: response.status(),
            message: response.content().to_string(),
            span,
        }
        .into());
    }
    let content: OrganizationsResponse = serde_json::from_str(response.content())
        .map_err(|e| deserialize_error(e.to_string(), span))?;

    let org = content.items().first().unwrap().id();
    Ok(org.to_string())
}

// duration_to_golang_string creates a golang formatted string to use with timeouts. Unlike Golang
// strings it does not deal with fractional seconds, we do not need that accuracy.
pub fn duration_to_golang_string(duration: Duration) -> String {
    let mut total_secs = duration.as_secs();
    let secs = total_secs % 60;
    total_secs /= 60;
    let mut golang_string = format!("{}s", secs);
    if total_secs > 0 {
        let minutes = total_secs % 60;
        total_secs /= 60;
        golang_string = format!("{}m{}", minutes, golang_string);
        if total_secs > 0 {
            golang_string = format!("{}h{}", total_secs, golang_string)
        }
    }

    golang_string
}

#[derive(Clone, Debug, Default)]
pub struct NuValueMap {
    cols: Vec<String>,
    vals: Vec<Value>,
}

impl NuValueMap {
    pub fn add(&mut self, name: impl Into<String>, val: Value) {
        self.cols.push(name.into());
        self.vals.push(val);
    }

    pub fn add_i64(&mut self, name: impl Into<String>, val: i64, span: Span) {
        self.cols.push(name.into());
        self.vals.push(Value::Int {
            val,
            internal_span: span,
        });
    }

    pub fn add_string(&mut self, name: impl Into<String>, val: impl Into<String>, span: Span) {
        self.cols.push(name.into());
        self.vals.push(Value::String {
            val: val.into(),
            internal_span: span,
        });
    }

    pub fn add_bool(&mut self, name: impl Into<String>, val: bool, span: Span) {
        self.cols.push(name.into());
        self.vals.push(Value::Bool {
            val,
            internal_span: span,
        });
    }

    pub fn into_value(self, span: Span) -> Value {
        Value::Record {
            val: SharedCow::new(
                Record::from_raw_cols_vals(self.cols, self.vals, span, span).unwrap(),
            ),
            internal_span: span,
        }
    }

    pub fn into_pipeline_data(self, span: Span) -> PipelineData {
        Value::Record {
            val: SharedCow::new(
                Record::from_raw_cols_vals(self.cols, self.vals, span, span).unwrap(),
            ),
            internal_span: span,
        }
        .into_pipeline_data()
    }
}

pub fn get_active_cluster<'a>(
    identifier: String,
    guard: &'a MutexGuard<State>,
    span: Span,
) -> Result<&'a RemoteCluster, ShellError> {
    match guard.clusters().get(&identifier) {
        Some(c) => Ok(c),
        None => Err(cluster_not_found_error(identifier, span)),
    }
}

#[cfg(test)]
mod tests {
    use crate::cli::util::duration_to_golang_string;
    use std::time::Duration;

    #[test]
    fn duration_to_golang_string_some_seconds() {
        assert_eq!(
            "2s".to_string(),
            duration_to_golang_string(Duration::from_secs(2))
        );
    }

    #[test]
    fn duration_to_golang_string_some_seconds_and_minutes() {
        assert_eq!(
            "5m2s".to_string(),
            duration_to_golang_string(Duration::from_secs(302))
        );
    }

    #[test]
    fn duration_to_golang_string_some_seconds_and_minutes_and_hours() {
        assert_eq!(
            "1h5m2s".to_string(),
            duration_to_golang_string(Duration::from_secs(3902))
        );
    }
}
