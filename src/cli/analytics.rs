use crate::cli::error::{
    deserialize_error, malformed_response_error, unexpected_status_code_error,
};
use crate::cli::util::{
    cluster_identifiers_from, convert_row_to_nu_value, duration_to_golang_string,
    get_active_cluster,
};
use crate::client::{AnalyticsQueryRequest, HttpResponse};
use crate::state::State;
use crate::RemoteCluster;
use log::debug;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};
use std::collections::HashMap;
use std::ops::Add;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

#[derive(Clone)]
pub struct Analytics {
    state: Arc<Mutex<State>>,
}

impl Analytics {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for Analytics {
    fn name(&self) -> &str {
        "analytics"
    }

    fn signature(&self) -> Signature {
        Signature::build("analytics")
            .required("statement", SyntaxShape::String, "the analytics statement")
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
            .switch("with-meta", "Includes related metadata in the result", None)
            .named(
                "clusters",
                SyntaxShape::String,
                "the clusters which should be contacted",
                None,
            )
            .category(Category::Custom("couchbase".into()))
    }

    fn usage(&self) -> &str {
        "Performs an analytics query"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        run(self.state.clone(), engine_state, stack, call, input)
    }
}

fn run(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();
    let statement: String = call.req(engine_state, stack, 0)?;
    let span = call.head;

    let cluster_identifiers = cluster_identifiers_from(engine_state, stack, &state, call, true)?;

    let guard = state.lock().unwrap();

    let scope = call.get_flag(engine_state, stack, "scope")?;
    let with_meta = call.has_flag("with-meta");

    debug!("Running analytics query {}", &statement);

    let mut results: Vec<Value> = vec![];
    for identifier in cluster_identifiers {
        let active_cluster = get_active_cluster(identifier.clone(), &guard, span.clone())?;
        let bucket = call
            .get_flag(engine_state, stack, "bucket")?
            .or_else(|| active_cluster.active_bucket());
        let maybe_scope = bucket.map(|b| scope.clone().map(|s| (b, s))).flatten();

        let response = send_analytics_query(
            active_cluster,
            maybe_scope,
            statement.clone(),
            ctrl_c.clone(),
        )?;

        if with_meta {
            let content: serde_json::Value =
                serde_json::from_str(response.content()).map_err(|_e| {
                    unexpected_status_code_error(response.status(), response.content(), span)
                })?;
            results.push(convert_row_to_nu_value(&content, span, identifier.clone())?);
        } else {
            let content: HashMap<String, serde_json::Value> =
                serde_json::from_str(response.content())
                    .map_err(|e| deserialize_error(e.to_string(), span))?;
            if let Some(content_errors) = content.get("errors") {
                if let Some(arr) = content_errors.as_array() {
                    for result in arr {
                        results.push(convert_row_to_nu_value(result, span, identifier.clone())?);
                    }
                } else {
                    return Err(malformed_response_error(
                        "analytics rows not an array",
                        content_errors.to_string(),
                        span,
                    ));
                }
            } else if let Some(content_results) = content.get("results") {
                if let Some(arr) = content_results.as_array() {
                    dbg!(&arr);
                    for result in arr {
                        results.push(convert_row_to_nu_value(result, span, identifier.clone())?);
                    }
                } else {
                    return Err(malformed_response_error(
                        "analytics toplevel result not  an object",
                        content_results.to_string(),
                        span,
                    ));
                }
            } else {
                // Queries like "create dataset" can end up here.
                continue;
            };
        }
    }

    Ok(Value::List {
        vals: results,
        span,
    }
    .into_pipeline_data())
}

pub fn send_analytics_query(
    active_cluster: &RemoteCluster,
    scope: Option<(String, String)>,
    statement: impl Into<String>,
    ctrl_c: Arc<AtomicBool>,
) -> Result<HttpResponse, ShellError> {
    let response = active_cluster
        .cluster()
        .http_client()
        .analytics_query_request(
            AnalyticsQueryRequest::Execute {
                statement: statement.into(),
                scope,
                timeout: duration_to_golang_string(active_cluster.timeouts().analytics_timeout()),
            },
            Instant::now().add(active_cluster.timeouts().analytics_timeout()),
            ctrl_c,
        )?;

    Ok(response)
}
