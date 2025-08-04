use crate::cli::error::{
    client_error_to_shell_error, deserialize_error, unexpected_status_code_error,
};
use crate::cli::util::{cluster_identifiers_from, convert_row_to_nu_value, get_active_cluster};
use crate::client::AnalyticsQueryRequest;
use crate::state::State;
use nu_engine::command_prelude::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;
use tokio::time::Instant;

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
            .category(Category::Custom("couchbase".to_string()))
    }

    fn description(&self) -> &str {
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
    let signals = engine_state.signals().clone();
    let span = call.head;

    let cluster_identifiers = cluster_identifiers_from(engine_state, stack, &state, call, true)?;
    let guard = state.lock().unwrap();

    let mut results: Vec<Value> = vec![];
    for identifier in cluster_identifiers {
        let active_cluster = get_active_cluster(identifier.clone(), &guard, span)?;
        let response = active_cluster
            .cluster()
            .http_client()
            .analytics_query_request(
                AnalyticsQueryRequest::PendingMutations,
                Instant::now().add(active_cluster.timeouts().analytics_timeout()),
                signals.clone(),
                Arc::new(Runtime::new().unwrap()),
            )
            .map_err(|e| client_error_to_shell_error(e, span))?;

        match response.status() {
            200 => {}
            _ => {
                return Err(unexpected_status_code_error(
                    response.status(),
                    response.content()?,
                    span,
                ));
            }
        }

        let content: serde_json::Value = serde_json::from_str(&response.content()?)
            .map_err(|e| deserialize_error(e.to_string(), span))?;
        let converted = &mut convert_row_to_nu_value(&content, span, identifier.clone())?;
        results.append(converted);
    }

    Ok(Value::List {
        vals: results,
        internal_span: span,
    }
    .into_pipeline_data())
}
