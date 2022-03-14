use crate::cli::util::{
    cluster_identifiers_from, cluster_not_found_error, convert_row_to_nu_value,
    map_serde_deserialize_error_to_shell_error, validate_is_not_cloud,
};
use crate::client::AnalyticsQueryRequest;
use crate::state::State;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct AnalyticsPendingMutations {
    state: Arc<Mutex<State>>,
}

impl AnalyticsPendingMutations {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for AnalyticsPendingMutations {
    fn name(&self) -> &str {
        "analytics pending-mutations"
    }

    fn signature(&self) -> Signature {
        Signature::build("analytics pending-mutations")
            .named(
                "clusters",
                SyntaxShape::String,
                "the clusters which should be contacted",
                None,
            )
            .category(Category::Custom("couchbase".into()))
    }

    fn usage(&self) -> &str {
        "Lists all analytics pending mutations"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        pending_mutations(self.state.clone(), engine_state, stack, call, input)
    }
}

fn pending_mutations(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();

    let cluster_identifiers = cluster_identifiers_from(engine_state, stack, &state, call, true)?;
    let guard = state.lock().unwrap();

    let mut results: Vec<Value> = vec![];
    for identifier in cluster_identifiers {
        let active_cluster = match guard.clusters().get(&identifier) {
            Some(c) => c,
            None => {
                return Err(cluster_not_found_error(identifier));
            }
        };
        validate_is_not_cloud(
            active_cluster,
            "pending mutations cannot be run against Capella clusters",
        )?;

        let response = active_cluster
            .cluster()
            .http_client()
            .analytics_query_request(
                AnalyticsQueryRequest::PendingMutations,
                Instant::now().add(active_cluster.timeouts().analytics_timeout()),
                ctrl_c.clone(),
            )?;

        let content: serde_json::Value = serde_json::from_str(response.content())
            .map_err(map_serde_deserialize_error_to_shell_error)?;
        let converted = convert_row_to_nu_value(&content, call.head, identifier.clone())?;
        results.push(converted);
    }

    Ok(Value::List {
        vals: results,
        span: call.head,
    }
    .into_pipeline_data())
}
