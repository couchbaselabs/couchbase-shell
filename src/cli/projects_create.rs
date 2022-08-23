use crate::cli::cloud_json::JSONCloudCreateProjectRequest;
use crate::client::CapellaRequest;
use crate::state::State;
use log::debug;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

use crate::cli::error::{serialize_error, unexpected_status_code_error};
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

    let guard = state.lock().unwrap();
    let control = guard.active_capella_org()?;
    let client = control.client();
    let project = JSONCloudCreateProjectRequest::new(name);
    let response = client.capella_request(
        CapellaRequest::CreateProject {
            payload: serde_json::to_string(&project)
                .map_err(|e| serialize_error(e.to_string(), span))?,
        },
        Instant::now().add(control.timeout()),
        ctrl_c,
    )?;
    if response.status() != 201 {
        return Err(unexpected_status_code_error(
            response.status(),
            response.content(),
            span,
        ));
    };

    return Ok(PipelineData::new(span));
}
