use crate::cli::util::find_project_id;
use crate::client::CapellaRequest;
use crate::state::State;
use log::debug;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

use crate::cli::error::unexpected_status_code_error;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
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

    fn usage(&self) -> &str {
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
    let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();

    let name: String = call.req(engine_state, stack, 0)?;

    debug!("Running projects drop for {}", &name);

    let guard = state.lock().unwrap();
    let control = guard.active_capella_org()?;
    let client = control.client();
    let deadline = Instant::now().add(control.timeout());
    let project_id = find_project_id(ctrl_c.clone(), name, &client, deadline, span.clone())?;

    let response = client.capella_request(
        CapellaRequest::DeleteProject {
            project_id: project_id.to_string(),
        },
        deadline,
        ctrl_c,
    )?;
    if response.status() != 204 {
        return Err(unexpected_status_code_error(
            response.status(),
            response.content(),
            span,
        ));
    };

    return Ok(PipelineData::new(span));
}
