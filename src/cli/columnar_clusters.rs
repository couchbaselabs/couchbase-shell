use crate::cli::client_error_to_shell_error;
use crate::cli::util::{convert_json_value_to_nu_value, find_org_id, find_project_id, NuValueMap};
use crate::state::State;
use nu_engine::command_prelude::Call;
use nu_engine::CallExt;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct ColumnarClusters {
    state: Arc<Mutex<State>>,
}

impl ColumnarClusters {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for ColumnarClusters {
    fn name(&self) -> &str {
        "columnar clusters"
    }

    fn signature(&self) -> Signature {
        Signature::build("columnar clusters")
            .named(
                "organization",
                SyntaxShape::String,
                "the Capella organization to use",
                None,
            )
            .named(
                "project",
                SyntaxShape::String,
                "the Capella project to use",
                None,
            )
            .switch("details", "return Columnar cluster details", None)
            .category(Category::Custom("couchbase".to_string()))
    }

    fn usage(&self) -> &str {
        "Lists all Columnar analytics clusters in the active Capella project"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        columnar_clusters(self.state.clone(), engine_state, stack, call, input)
    }
}

fn columnar_clusters(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let guard = state.lock().unwrap();
    let signals = engine_state.signals().clone();

    let control =
        guard.named_or_active_org(call.get_flag(engine_state, stack, "organization")?)?;

    let project =
        guard.named_or_active_project(call.get_flag(engine_state, stack, "project")?)?;
    let client = control.client();

    let org_id = find_org_id(signals.clone(), &client, span)?;
    let project_id = find_project_id(signals.clone(), project, &client, span, org_id.clone())?;

    let clusters = client
        .list_columnar_clusters(org_id, project_id, signals)
        .map_err(|e| client_error_to_shell_error(e, span))?;

    let detail = call.has_flag(engine_state, stack, "details")?;

    let mut results = vec![];
    for cluster in clusters.items() {
        let mut collected = NuValueMap::default();
        collected.add_string("name", cluster.name(), span);
        collected.add_string("id", cluster.id(), span);
        collected.add_string("state", cluster.state(), span);
        collected.add_i64("number of nodes", cluster.nodes(), span);
        collected.add_string("provider", cluster.provider(), span);
        collected.add_string("region", cluster.region(), span);

        if detail {
            if let Some(desc) = cluster.description() {
                collected.add_string("description", desc, span);
            }

            collected.add(
                "compute",
                convert_json_value_to_nu_value(
                    &serde_json::to_value(cluster.compute()).unwrap(),
                    span,
                )
                .unwrap(),
            );
            collected.add(
                "availability",
                convert_json_value_to_nu_value(
                    &serde_json::to_value(cluster.availability()).unwrap(),
                    span,
                )
                .unwrap(),
            );
            collected.add(
                "support",
                convert_json_value_to_nu_value(
                    &serde_json::to_value(cluster.support()).unwrap(),
                    span,
                )
                .unwrap(),
            );
        }

        results.push(collected.into_value(span))
    }

    Ok(Value::List {
        vals: results,
        internal_span: span,
    }
    .into_pipeline_data())
}
