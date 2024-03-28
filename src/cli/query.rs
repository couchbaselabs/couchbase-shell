use crate::cli::util::{
    cluster_identifiers_from, convert_row_to_nu_value, duration_to_golang_string,
    get_active_cluster, is_http_status,
};
use crate::client::{HttpResponse, QueryRequest, QueryTransactionRequest};
use crate::state::State;
use log::debug;
use std::collections::HashMap;
use std::ops::Add;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::Instant;

use crate::cli::error::{
    client_error_to_shell_error, deserialize_error, malformed_response_error, query_error,
    QueryErrorReason,
};
use crate::RemoteCluster;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct Query {
    state: Arc<Mutex<State>>,
}

impl Query {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for Query {
    fn name(&self) -> &str {
        "query"
    }

    fn signature(&self) -> Signature {
        Signature::build("query")
            .required("statement", SyntaxShape::String, "the query statement")
            .named(
                "databases",
                SyntaxShape::String,
                "the databases to query against",
                None,
            )
            .named(
                "bucket",
                SyntaxShape::String,
                "the bucket to query against",
                None,
            )
            .named(
                "scope",
                SyntaxShape::String,
                "the scope to query against",
                None,
            )
            .switch("with-meta", "include toplevel metadata", None)
            .switch("disable-context", "disable automatically detecting the query context based on the active bucket and scope", None)
            .category(Category::Custom("couchbase".to_string()))
    }

    fn usage(&self) -> &str {
        "Performs a n1ql query"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        query(self.state.clone(), engine_state, stack, call, input)
    }
}

fn query(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();

    let cluster_identifiers = cluster_identifiers_from(engine_state, stack, &state, call, true)?;

    let statement: String = call.req(engine_state, stack, 0)?;

    let mut results: Vec<Value> = vec![];
    for identifier in cluster_identifiers {
        let guard = state.lock().unwrap();
        let active_cluster = get_active_cluster(identifier.clone(), &guard, span)?;

        let maybe_scope = query_context_from_args(active_cluster, engine_state, stack, call)?;

        debug!("Running n1ql query {}", &statement);

        let response = send_query(
            active_cluster,
            statement.clone(),
            maybe_scope,
            ctrl_c.clone(),
            None,
            span,
            None,
        )?;
        drop(guard);

        results.extend(handle_query_response(
            call.has_flag(engine_state, stack, "with-meta")?,
            identifier.clone(),
            response,
            span,
        )?);
    }

    if results.len() > 0 {
        return Ok(Value::List {
            vals: results,
            internal_span: call.head,
        }
        .into_pipeline_data());
    }

    Ok(PipelineData::new_with_metadata(None, span))
}

pub fn send_query(
    cluster: &RemoteCluster,
    statement: impl Into<String>,
    scope: Option<(String, String)>,
    ctrl_c: Arc<AtomicBool>,
    timeout: impl Into<Option<Duration>>,
    span: Span,
    transaction: impl Into<Option<QueryTransactionRequest>>,
) -> Result<HttpResponse, ShellError> {
    let timeout = timeout.into().unwrap_or(cluster.timeouts().query_timeout());
    let response = cluster
        .cluster()
        .http_client()
        .query_request(
            QueryRequest::Execute {
                statement: statement.into(),
                scope,
                timeout: duration_to_golang_string(timeout),
                transaction: transaction.into(),
            },
            Instant::now().add(timeout),
            ctrl_c,
        )
        .map_err(|e| client_error_to_shell_error(e, span))?;

    Ok(response)
}

pub fn handle_query_response(
    with_meta: bool,
    identifier: String,
    response: HttpResponse,
    span: Span,
) -> Result<Vec<Value>, ShellError> {
    is_http_status(&response, 200, span)?;

    let mut results: Vec<Value> = vec![];
    if with_meta {
        let content: serde_json::Value = serde_json::from_str(response.content())
            .map_err(|e| deserialize_error(e.to_string(), span))?;
        results.push(convert_row_to_nu_value(&content, span, identifier)?);
    } else {
        let content: HashMap<String, serde_json::Value> = serde_json::from_str(response.content())
            .map_err(|e| deserialize_error(e.to_string(), span))?;
        if let Some(content_errors) = content.get("errors") {
            return if let Some(arr) = content_errors.as_array() {
                if arr.len() == 1 {
                    let e = match arr.get(0) {
                        Some(e) => e,
                        None => {
                            return Err(malformed_response_error(
                                "query errors present but empty",
                                content_errors.to_string(),
                                span,
                            ))
                        }
                    };
                    let code = e.get("code").map(|c| c.as_i64().unwrap_or_default());
                    let reason = match code {
                        Some(c) => QueryErrorReason::from(c),
                        None => QueryErrorReason::UnknownError,
                    };
                    let msg = match e.get("msg") {
                        Some(msg) => msg.to_string(),
                        None => "".to_string(),
                    };
                    Err(query_error(reason, code, msg, span))
                } else {
                    let messages = arr
                        .into_iter()
                        .map(|e| e.to_string())
                        .collect::<Vec<String>>()
                        .join(",");

                    Err(query_error(
                        QueryErrorReason::MultiErrors,
                        None,
                        messages,
                        span,
                    ))
                }
            } else {
                Err(malformed_response_error(
                    "query errors not an array",
                    content_errors.to_string(),
                    span,
                ))
            };
        } else if let Some(content_results) = content.get("results") {
            if let Some(arr) = content_results.as_array() {
                for result in arr {
                    results
                        .push(convert_row_to_nu_value(result, span, identifier.clone()).unwrap());
                }
            } else {
                return Err(malformed_response_error(
                    "query results not an array",
                    content_results.to_string(),
                    span,
                ));
            }
        } else {
            // Queries like "create index" can end up here.
        };
    }

    Ok(results)
}

pub fn query_context_from_args(
    cluster: &RemoteCluster,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<Option<(String, String)>, ShellError> {
    let bucket = call
        .get_flag(engine_state, stack, "bucket")?
        .or_else(|| cluster.active_bucket());

    let scope = call
        .get_flag(engine_state, stack, "scope")?
        .or_else(|| cluster.active_scope());

    let disable_context = call.has_flag(engine_state, stack, "disable-context")?;

    Ok(if disable_context {
        None
    } else {
        bucket.and_then(|b| scope.map(|s| (b, s)))
    })
}
