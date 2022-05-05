use crate::cli::util::{
    cluster_identifiers_from, cluster_not_found_error, convert_row_to_nu_value,
    duration_to_golang_string, generic_unspanned_error, map_serde_deserialize_error_to_shell_error,
};
use crate::client::AnalyticsQueryRequest;
use crate::state::State;
use log::debug;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct AnalyticsIndexes {
    state: Arc<Mutex<State>>,
}

impl AnalyticsIndexes {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for AnalyticsIndexes {
    fn name(&self) -> &str {
        "analytics indexes"
    }

    fn signature(&self) -> Signature {
        Signature::build("analytics indexes")
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
        "Lists all analytics indexes"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        indexes(self.state.clone(), engine_state, stack, call, input)
    }
}

fn indexes(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();
    let statement = "SELECT d.* FROM Metadata.`Index` d WHERE d.DataverseName <> \"Metadata\"";

    let cluster_identifiers = cluster_identifiers_from(engine_state, stack, &state, call, true)?;
    let guard = state.lock().unwrap();
    debug!("Running analytics query {}", &statement);

    let mut results: Vec<Value> = vec![];
    for identifier in cluster_identifiers {
        let active_cluster = match guard.clusters().get(&identifier) {
            Some(c) => c,
            None => {
                return Err(cluster_not_found_error(identifier, call.span()));
            }
        };

        let response = active_cluster
            .cluster()
            .http_client()
            .analytics_query_request(
                AnalyticsQueryRequest::Execute {
                    statement: statement.into(),
                    scope: None,
                    timeout: duration_to_golang_string(
                        active_cluster.timeouts().analytics_timeout(),
                    ),
                },
                Instant::now().add(active_cluster.timeouts().analytics_timeout()),
                ctrl_c.clone(),
            )?;

        let with_meta = call.has_flag("with-meta");
        let content: serde_json::Value = serde_json::from_str(response.content())
            .map_err(map_serde_deserialize_error_to_shell_error)?;
        if with_meta {
            let converted = convert_row_to_nu_value(&content, call.head, identifier.clone())?;
            results.push(converted);
        } else if let Some(content_results) = content.get("results") {
            if let Some(arr) = content_results.as_array() {
                let mut converted = vec![];
                for result in arr {
                    converted.push(convert_row_to_nu_value(
                        result,
                        call.head,
                        identifier.clone(),
                    )?);
                }
            } else {
                return Err(generic_unspanned_error(
                    "Analytics results not an array - malformed response",
                    format!(
                        "Analytics results not an array - {}",
                        content_results.to_string(),
                    ),
                ));
            }
        } else {
            return Err(generic_unspanned_error(
                "Analytics toplevel result not  an object",
                format!(
                    "Analytics toplevel result not  an object - {}",
                    content.to_string(),
                ),
            ));
        }
    }

    Ok(Value::List {
        vals: results,
        span: call.head,
    }
    .into_pipeline_data())
}
