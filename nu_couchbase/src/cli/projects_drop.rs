use crate::cli::util::{find_org_id, find_project_id};
use crate::state::State;
use log::debug;
use std::sync::{Arc, Mutex};

use crate::cli::error::client_error_to_shell_error;
use nu_engine::command_prelude::Call;
use nu_engine::CallExt;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape};

#[derive(Clone)]
pub struct ProjectsDrop {
    state: Arc<Mutex<State>>,
}

impl ProjectsDrop {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for ProjectsDrop {
    fn name(&self) -> &str {
        "projects drop"
    }

    fn signature(&self) -> Signature {
        Signature::build("projects drop")
            .required("name", SyntaxShape::String, "the name of the project")
            .category(Category::Custom("couchbase".to_string()))
    }

    fn description(&self) -> &str {
        "Deletes a Capella project"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        projects_drop(self.state.clone(), engine_state, stack, call, input)
    }
}

fn projects_drop(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let signals = engine_state.signals().clone();

    let name: String = call.req(engine_state, stack, 0)?;

    debug!("Running projects drop for {}", &name);

    let guard = &mut state.lock().unwrap();
    let control = guard.active_capella_org()?;
    let client = control.client();

    let org_id = find_org_id(signals.clone(), &client, span)?;
    let project_id = find_project_id(signals.clone(), name, &client, span, org_id.clone())?;

    client
        .delete_project(org_id, project_id, signals)
        .map_err(|e| client_error_to_shell_error(e, span))?;

    Ok(PipelineData::empty())
}
