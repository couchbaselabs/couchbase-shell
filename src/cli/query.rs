use crate::cli::util::{
    cluster_identifiers_from, convert_json_value_to_nu_value, convert_row_to_nu_value,
    duration_to_golang_string, get_active_cluster,
};
use crate::client::{HttpResponse, QueryRequest};
use crate::state::State;
use log::debug;
use std::collections::HashMap;
use std::ops::Add;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

use crate::cli::error::{
    deserialize_error, malformed_response_error, unexpected_status_code_error,
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
                "clusters",
                SyntaxShape::String,
                "the clusters to query against",
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
            .category(Category::Custom("couchbase".into()))
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

    let cluster_identifiers = cluster_identifiers_from(&engine_state, stack, &state, &call, true)?;

    let statement: String = call.req(engine_state, stack, 0)?;

    let mut results: Vec<Value> = vec![];
    for identifier in cluster_identifiers {
        let guard = state.lock().unwrap();
        let active_cluster = get_active_cluster(identifier.clone(), &guard, span.clone())?;

        let maybe_scope = query_context_from_args(active_cluster, engine_state, stack, call)?;

        debug!("Running n1ql query {}", &statement);

        let response = send_query(
            active_cluster,
            statement.clone(),
            maybe_scope,
            ctrl_c.clone(),
            span.clone(),
        )?;
        drop(guard);

        results.extend(handle_query_response(
            call.has_flag("with-meta"),
            identifier.clone(),
            response,
            span.clone(),
        )?);
    }

    Ok(Value::List {
        vals: results,
        span: call.head,
    }
    .into_pipeline_data())
}

pub fn send_query(
    cluster: &RemoteCluster,
    statement: String,
    scope: Option<(String, String)>,
    ctrl_c: Arc<AtomicBool>,
    span: Span,
) -> Result<HttpResponse, ShellError> {
    let response = cluster.cluster().http_client().query_request(
        QueryRequest::Execute {
            statement,
            scope,
            timeout: duration_to_golang_string(cluster.timeouts().query_timeout()),
        },
        Instant::now().add(cluster.timeouts().query_timeout()),
        ctrl_c,
    )?;

    match response.status() {
        200 => {}
        _ => {
            return Err(unexpected_status_code_error(
                response.status(),
                response.content(),
                span,
            ));
        }
    }

    Ok(response)
}

pub fn handle_query_response(
    with_meta: bool,
    identifier: String,
    response: HttpResponse,
    span: Span,
) -> Result<Vec<Value>, ShellError> {
    let mut results: Vec<Value> = vec![];
    if with_meta {
        let content: serde_json::Value = serde_json::from_str(response.content())
            .map_err(|e| deserialize_error(e.to_string(), span))?;
        results.push(convert_row_to_nu_value(&content, span, identifier)?);
    } else {
        let content: HashMap<String, serde_json::Value> = serde_json::from_str(response.content())
            .map_err(|e| deserialize_error(e.to_string(), span))?;
        if let Some(content_errors) = content.get("errors") {
            if let Some(arr) = content_errors.as_array() {
                for result in arr {
                    results.push(convert_row_to_nu_value(result, span, identifier.clone())?);
                }
            } else {
                return Err(malformed_response_error(
                    "query errors not an array",
                    content_errors.to_string(),
                    span,
                ));
            }
        } else if let Some(content_results) = content.get("results") {
            if let Some(arr) = content_results.as_array() {
                for result in arr {
                    results.push(convert_json_value_to_nu_value(result, span).unwrap());
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

    let disable_context = call.has_flag("disable-context");

    Ok(if disable_context {
        None
    } else {
        bucket.map(|b| scope.map(|s| (b, s))).flatten()
    })
}
