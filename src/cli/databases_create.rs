use crate::cli::cloud_json::JSONCloudCreateClusterRequestV3;
use crate::cli::util::find_project_id;
use crate::client::CapellaRequest;
use crate::state::State;
use log::debug;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

use crate::cli::error::{
    client_error_to_shell_error, no_active_project_error, serialize_error,
    unexpected_status_code_error,
};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape};

#[derive(Clone)]
pub struct DatabasesCreate {
    state: Arc<Mutex<State>>,
}

impl DatabasesCreate {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for DatabasesCreate {
    fn name(&self) -> &str {
        "databases create"
    }

    fn signature(&self) -> Signature {
        Signature::build("databases create")
            .required(
                "definition",
                SyntaxShape::String,
                "the definition of the cluster",
            )
            .named(
                "capella",
                SyntaxShape::String,
                "the Capella organization to use",
                None,
            )
            .category(Category::Custom("couchbase".to_string()))
    }

    fn usage(&self) -> &str {
        "Creates a new database on the active Capella organization"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        clusters_create(self.state.clone(), engine_state, stack, call, input)
    }
}

fn clusters_create(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();

    let definition: String = call.req(engine_state, stack, 0)?;
    let capella = call.get_flag(engine_state, stack, "capella")?;

    debug!("Running databases create for {}", &definition);

    let guard = state.lock().unwrap();
    let control = if let Some(c) = capella {
        guard.capella_org_for_cluster(c)
    } else {
        guard.active_capella_org()
    }?;
    let client = control.client();
    let deadline = Instant::now().add(control.timeout());

    let project_name = match control.active_project() {
        Some(p) => p,
        None => {
            return Err(no_active_project_error(span));
        }
    };
    let project_id = find_project_id(ctrl_c.clone(), project_name, &client, deadline, span)?;

    let mut json: JSONCloudCreateClusterRequestV3 = serde_json::from_str(definition.as_str())
        .map_err(|e| serialize_error(e.to_string(), span))?;
    json.set_project_id(project_id);

    let response = client
        .capella_request(
            CapellaRequest::CreateClusterV3 {
                payload: serde_json::to_string(&json)
                    .map_err(|e| serialize_error(e.to_string(), span))?,
            },
            Instant::now().add(control.timeout()),
            ctrl_c,
        )
        .map_err(|e| client_error_to_shell_error(e, span))?;
    if response.status() != 202 {
        return Err(unexpected_status_code_error(
            response.status(),
            response.content(),
            span,
        ));
    };

    Ok(PipelineData::new_with_metadata(None, span))
}
