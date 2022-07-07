use crate::cli::analytics::send_analytics_query;
use crate::cli::error::{malformed_response_error, unexpected_status_code_error};
use crate::cli::util::{cluster_identifiers_from, convert_row_to_nu_value, get_active_cluster};
use crate::state::State;
use crate::RemoteCluster;
use log::debug;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct AnalyticsBuckets {
    state: Arc<Mutex<State>>,
}

impl AnalyticsBuckets {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for AnalyticsBuckets {
    fn name(&self) -> &str {
        "analytics buckets"
    }

    fn signature(&self) -> Signature {
        Signature::build("analytics buckets")
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
        "Lists all analytics buckets"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        dataverses(self.state.clone(), engine_state, stack, call, input)
    }
}

fn dataverses(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();
    let statement = "SELECT `Bucket`.* FROM `Metadata`.`Bucket`";
    let span = call.head;

    let with_meta = call.has_flag("with-meta");

    let cluster_identifiers = cluster_identifiers_from(engine_state, stack, &state, call, true)?;

    let guard = state.lock().unwrap();
    debug!("Running analytics query {}", &statement);

    let mut results: Vec<Value> = vec![];
    for identifier in cluster_identifiers {
        let active_cluster = get_active_cluster(identifier.clone(), &guard, span.clone())?;

        results.extend(do_non_mutation_analytics_query(
            identifier.clone(),
            active_cluster,
            statement.clone(),
            ctrl_c.clone(),
            span.clone(),
            with_meta,
        )?);
    }

    Ok(Value::List {
        vals: results,
        span,
    }
    .into_pipeline_data())
}

pub fn do_non_mutation_analytics_query(
    identifier: String,
    active_cluster: &RemoteCluster,
    statement: impl Into<String>,
    ctrl_c: Arc<AtomicBool>,
    span: Span,
    with_meta: bool,
) -> Result<Vec<Value>, ShellError> {
    let response = send_analytics_query(active_cluster, None, statement, ctrl_c)?;

    let content: serde_json::Value = serde_json::from_str(response.content())
        .map_err(|_e| unexpected_status_code_error(response.status(), response.content(), span))?;

    let mut results: Vec<Value> = vec![];
    if with_meta {
        let converted = convert_row_to_nu_value(&content, span.clone(), identifier.clone())?;
        results.push(converted);
        return Ok(results);
    }

    if let Some(content_results) = content.get("results") {
        if let Some(arr) = content_results.as_array() {
            for result in arr {
                results.push(convert_row_to_nu_value(
                    result,
                    span.clone(),
                    identifier.clone(),
                )?);
            }
        } else {
            return Err(malformed_response_error(
                "analytics rows not an array",
                content_results.to_string(),
                span,
            ));
        }
    } else {
        return Err(malformed_response_error(
            "analytics toplevel result not an object",
            content.to_string(),
            span,
        ));
    }

    Ok(results)
}
