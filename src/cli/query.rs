use crate::cli::util::{
    cluster_identifiers_from, cluster_not_found_error, convert_json_value_to_nu_value,
    convert_row_to_nu_value, duration_to_golang_string, generic_unspanned_error,
    map_serde_deserialize_error_to_shell_error,
};
use crate::client::QueryRequest;
use crate::state::State;
use log::debug;
use std::collections::HashMap;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
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

    let guard = state.lock().unwrap();
    let statement: String = call.req(engine_state, stack, 0)?;

    let mut results: Vec<Value> = vec![];
    for identifier in cluster_identifiers {
        let active_cluster = match guard.clusters().get(&identifier) {
            Some(c) => c,
            None => {
                return Err(cluster_not_found_error(identifier, call.span()));
            }
        };
        let bucket = call
            .get_flag(engine_state, stack, "bucket")?
            .or_else(|| active_cluster.active_bucket());

        let scope = call.get_flag(engine_state, stack, "scope")?;

        let maybe_scope = bucket.map(|b| scope.map(|s| (b, s))).flatten();

        let with_meta = call.has_flag("with-meta");

        debug!("Running n1ql query {}", &statement);

        let response = active_cluster.cluster().http_client().query_request(
            QueryRequest::Execute {
                statement: statement.clone(),
                scope: maybe_scope,
                timeout: duration_to_golang_string(active_cluster.timeouts().query_timeout()),
            },
            Instant::now().add(active_cluster.timeouts().query_timeout()),
            ctrl_c.clone(),
        )?;

        if with_meta {
            let content: serde_json::Value = serde_json::from_str(response.content())
                .map_err(map_serde_deserialize_error_to_shell_error)?;
            results.push(convert_row_to_nu_value(&content, span, identifier.clone())?);
        } else {
            let content: HashMap<String, serde_json::Value> =
                serde_json::from_str(response.content())
                    .map_err(map_serde_deserialize_error_to_shell_error)?;
            if let Some(content_errors) = content.get("errors") {
                if let Some(arr) = content_errors.as_array() {
                    for result in arr {
                        results.push(convert_row_to_nu_value(result, span, identifier.clone())?);
                    }
                } else {
                    return Err(generic_unspanned_error(
                        "Query errors not an array - malformed response",
                        format!("Query errors not an array - {}", content_errors.to_string(),),
                    ));
                }
            } else if let Some(content_results) = content.get("results") {
                if let Some(arr) = content_results.as_array() {
                    for result in arr {
                        results.push(convert_json_value_to_nu_value(result, span).unwrap());
                    }
                } else {
                    return Err(generic_unspanned_error(
                        "Query results not an array - malformed response",
                        format!(
                            "Query results not an array - {}",
                            content_results.to_string(),
                        ),
                    ));
                }
            } else {
                // Queries like "create index" can end up here.
                continue;
            };
        }
    }

    Ok(Value::List {
        vals: results,
        span: call.head,
    }
    .into_pipeline_data())
}
