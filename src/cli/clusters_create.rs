use crate::cli::cloud_json::JSONCloudCreateClusterRequestV4;
use crate::client::CapellaRequest;
use crate::state::State;
use log::debug;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

use crate::cli::error::{
    client_error_to_shell_error, serialize_error, unexpected_status_code_error,
};
use crate::cli::util::{find_org_id, find_project_id};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape};

#[derive(Clone)]
pub struct ClustersCreate {
    state: Arc<Mutex<State>>,
}

impl ClustersCreate {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for ClustersCreate {
    fn name(&self) -> &str {
        "clusters create"
    }

    fn signature(&self) -> Signature {
        Signature::build("clusters create")
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
        "Creates a new cluster on the active Capella organization"
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

    debug!("Running clusters create for {}", &definition);

    let guard = state.lock().unwrap();
    let control = if let Some(c) = capella {
        guard.get_capella_org(c)
    } else {
        guard.active_capella_org()
    }?;
    let client = control.client();
    let deadline = Instant::now().add(control.timeout());

    let org_id = find_org_id(ctrl_c.clone(), &client, deadline, span)?;
    let project_id = find_project_id(
        ctrl_c.clone(),
        guard.active_project()?,
        &client,
        deadline,
        span,
        org_id.clone(),
    )?;

    let json: JSONCloudCreateClusterRequestV4 = serde_json::from_str(definition.as_str())
        .map_err(|e| serialize_error(e.to_string(), span))?;

    let response = client
        .capella_request(
            CapellaRequest::CreateClusterV4 {
                org_id,
                project_id,
                payload: serde_json::to_string(&json)
                    .map_err(|e| serialize_error(e.to_string(), span))?,
            },
            deadline,
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

    Ok(PipelineData::empty())
}
