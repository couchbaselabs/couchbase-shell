use crate::cli::client_error_to_shell_error;
use crate::cli::util::{find_org_id, find_project_id};
use crate::state::State;
use nu_engine::command_prelude::Call;
use nu_engine::CallExt;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct ColumnarClustersDrop {
    state: Arc<Mutex<State>>,
}

impl ColumnarClustersDrop {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for ColumnarClustersDrop {
    fn name(&self) -> &str {
        "columnar clusters drop"
    }

    fn signature(&self) -> Signature {
        Signature::build("columnar clusters drop")
            .required("name", SyntaxShape::String, "the name of the cluster")
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
            .category(Category::Custom("couchbase".to_string()))
    }

    fn description(&self) -> &str {
        "Deletes a Columnar analytics clusters from the active Capella organization"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        columnar_clusters_drop(self.state.clone(), engine_state, stack, call, input)
    }
}
fn columnar_clusters_drop(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let name: String = call.req(engine_state, stack, 0)?;
    let signals = engine_state.signals().clone();

    let guard = state.lock().unwrap();
    let control =
        guard.named_or_active_org(call.get_flag(engine_state, stack, "organization")?)?;

    let project =
        guard.named_or_active_project(call.get_flag(engine_state, stack, "project")?)?;
    let client = control.client();

    let org_id = find_org_id(signals.clone(), &client, span)?;
    let project_id = find_project_id(signals.clone(), project, &client, span, org_id.clone())?;

    let cluster = client
        .get_columnar_cluster(name, org_id.clone(), project_id.clone(), signals.clone())
        .map_err(|e| client_error_to_shell_error(e, span))?;

    client
        .delete_columnar_cluster(org_id, project_id, cluster.id(), signals)
        .map_err(|e| client_error_to_shell_error(e, span))?;

    Ok(PipelineData::empty())
}
