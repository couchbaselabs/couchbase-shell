use crate::cli::analytics_buckets::do_non_mutation_analytics_query;
use crate::cli::util::{cluster_identifiers_from, get_active_cluster};
use crate::state::State;
use log::debug;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};
use std::sync::{Arc, Mutex};

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

    fn usage(&self) -> &str {
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
    let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();
    let statement = "SELECT d.* FROM Metadata.`Dataset` d WHERE d.DataverseName <> \"Metadata\"";
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
