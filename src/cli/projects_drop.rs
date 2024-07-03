use crate::cli::util::find_org_id;
use crate::cli::util::find_project_id;
use crate::client::CapellaRequest;
use crate::state::State;
use log::debug;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

use crate::cli::error::{client_error_to_shell_error, unexpected_status_code_error};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape};
use nu_protocol::Value::Nothing;

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

    let guard = &mut state.lock().unwrap();
    let control = guard.active_capella_org()?;
    let client = control.client();
    let deadline = Instant::now().add(control.timeout());

    let org_id = match control.id() {
        Some(id) => id,
        None => {
            let id = find_org_id(ctrl_c.clone(), &client, deadline, span)?;
            guard.set_active_capella_org_id(id.clone())?;
            id
        }
    };

    let project_id = find_project_id(
        ctrl_c.clone(),
        name,
        &client,
        deadline,
        span,
        org_id.clone(),
    )?;

    let response = client
        .capella_request(
            CapellaRequest::DeleteProject { org_id, project_id },
            deadline,
            ctrl_c,
        )
        .map_err(|e| client_error_to_shell_error(e, span))?;
    if response.status() != 204 {
        return Err(unexpected_status_code_error(
            response.status(),
            response.content(),
            span,
        ));
    };

    Ok(PipelineData::Value(Nothing {internal_span: span}, None))
}
