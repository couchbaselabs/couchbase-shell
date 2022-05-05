use crate::cli::cloud_json::{JSONCloudCreateClusterRequest, JSONCloudCreateClusterRequestV3};
use crate::cli::util::generic_unspanned_error;
use crate::cli::util::{find_cloud_id, find_project_id, map_serde_serialize_error_to_shell_error};
use crate::client::CapellaRequest;
use crate::state::State;
use log::debug;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

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
            .named(
                "environment",
                SyntaxShape::String,
                "the Capella environment to use (\"hosted\" or \"vpc\")",
                None,
            )
            .category(Category::Custom("couchbase".into()))
    }

    fn usage(&self) -> &str {
        "Creates a new cluster against the active Capella organization"
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
    let environment = call
        .get_flag(engine_state, stack, "environment")?
        .unwrap_or("hosted".to_string());

    debug!("Running clusters create for {}", &definition);

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
            return Err(ShellError::MissingParameter(
                "Could not auto-select a project, set an active project".into(),
                span,
            ))
        }
    };
    let project_id = find_project_id(ctrl_c.clone(), project_name, &client, deadline)?;

    if environment == "hosted".to_string() {
        let mut json: JSONCloudCreateClusterRequestV3 =
            serde_json::from_str(definition.as_str())
                .map_err(map_serde_serialize_error_to_shell_error)?;
        json.set_project_id(project_id);

        let response = client.capella_request(
            CapellaRequest::CreateClusterV3 {
                payload: serde_json::to_string(&json)
                    .map_err(map_serde_serialize_error_to_shell_error)?,
            },
            Instant::now().add(control.timeout()),
            ctrl_c,
        )?;
        if response.status() != 202 {
            return Err(generic_unspanned_error(
                "Failed to create cluster",
                format!("Failed to create cluster {}", response.content()),
            ));
        };

        return Ok(PipelineData::new(span));
    }

    let cloud_name = match control.active_cloud() {
        Some(p) => p,
        None => {
            return Err(ShellError::MissingParameter(
                "Could not auto-select a cloud, set an active cloud".into(),
                span,
            ))
        }
    };
    let cloud_id = find_cloud_id(ctrl_c.clone(), cloud_name, &client, deadline)?;

    let mut json: JSONCloudCreateClusterRequest = serde_json::from_str(definition.as_str())
        .map_err(map_serde_serialize_error_to_shell_error)?;
    json.set_cloud_id(cloud_id);
    json.set_project_id(project_id);

    let response = client.capella_request(
        CapellaRequest::CreateCluster {
            payload: serde_json::to_string(&json)
                .map_err(map_serde_serialize_error_to_shell_error)?,
        },
        Instant::now().add(control.timeout()),
        ctrl_c,
    )?;
    if response.status() != 202 {
        return Err(generic_unspanned_error(
            "Failed to create cluster",
            format!("Failed to create cluster {}", response.content()),
        ));
    };

    Ok(PipelineData::new(span))
}
