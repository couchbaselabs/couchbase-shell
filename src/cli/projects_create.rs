use crate::cli::util::find_org_id;
use crate::state::State;
use log::debug;
use std::sync::{Arc, Mutex};

use crate::cli::error::client_error_to_shell_error;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape};

#[derive(Clone)]
pub struct ProjectsCreate {
    state: Arc<Mutex<State>>,
}

impl ProjectsCreate {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for ProjectsCreate {
    fn name(&self) -> &str {
        "projects create"
    }

    fn signature(&self) -> Signature {
        Signature::build("projects create")
            .required("name", SyntaxShape::String, "The name of the project")
            .category(Category::Custom("couchbase".to_string()))
    }

    fn usage(&self) -> &str {
        "Creates a new Capella project"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        projects_create(self.state.clone(), engine_state, stack, call, input)
    }
}

fn projects_create(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();
    let name: String = call.req(engine_state, stack, 0)?;

    debug!("Running projects create for {}", &name);

    let guard = &mut state.lock().unwrap();
    let control = guard.active_capella_org()?;
    let client = control.client();

    let org_id = find_org_id(ctrl_c.clone(), &client, span)?;
    client
        .create_project(org_id, name, ctrl_c)
        .map_err(|e| client_error_to_shell_error(e, span))?;

    Ok(PipelineData::empty())
}
