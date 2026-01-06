use crate::cli::analytics_common::{read_analytics_response, send_analytics_query};
use crate::cli::util::{cluster_identifiers_from, get_active_cluster};
use crate::state::State;
use log::debug;
use nu_engine::command_prelude::Call;
use nu_engine::CallExt;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;

#[derive(Clone)]
pub struct AnalyticsDatasets {
    state: Arc<Mutex<State>>,
}

impl AnalyticsDatasets {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for AnalyticsDatasets {
    fn name(&self) -> &str {
        "analytics datasets"
    }

    fn signature(&self) -> Signature {
        Signature::build("analytics datasets")
            .switch("with-meta", "Includes related metadata in the result", None)
            .named(
                "clusters",
                SyntaxShape::String,
                "the clusters which should be contacted",
                None,
            )
            .category(Category::Custom("couchbase".to_string()))
    }

    fn description(&self) -> &str {
        "Lists all analytics datasets"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        datasets(self.state.clone(), engine_state, stack, call, input)
    }
}

fn datasets(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let signals = engine_state.signals().clone();
    let statement = "SELECT d.* FROM Metadata.`Dataset` d WHERE d.DataverseName <> \"Metadata\"";
    let span = call.head;

    let with_meta = call.has_flag(engine_state, stack, "with-meta")?;

    let cluster_identifiers = cluster_identifiers_from(engine_state, stack, &state, call, true)?;

    let guard = state.lock().unwrap();
    debug!("Running analytics query {}", &statement);

    let mut results: Vec<Value> = vec![];
    for identifier in cluster_identifiers {
        let active_cluster = get_active_cluster(identifier.clone(), &guard, span)?;
        let resp = send_analytics_query(
            active_cluster,
            None,
            statement,
            signals.clone(),
            span,
            Arc::new(Runtime::new().unwrap()),
        )?;

        results.extend(read_analytics_response(
            identifier.clone(),
            resp,
            span,
            with_meta,
            false,
        )?);
    }

    Ok(Value::list(results, span).into_pipeline_data())
}
