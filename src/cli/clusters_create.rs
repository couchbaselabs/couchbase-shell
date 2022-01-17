use crate::cli::cloud_json::{JSONCloudCreateClusterRequest, JSONCloudCreateClusterRequestV3};
use crate::cli::util::{find_cloud_id, find_project_id};
use crate::client::CapellaRequest;
use crate::state::State;
use async_trait::async_trait;
use log::debug;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_stream::OutputStream;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

pub struct ClustersCreate {
    state: Arc<Mutex<State>>,
}

impl ClustersCreate {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for ClustersCreate {
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
    }

    fn usage(&self) -> &str {
        "Creates a new cluster against the active Capella organization"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        clusters_create(self.state.clone(), args)
    }
}

fn clusters_create(
    state: Arc<Mutex<State>>,
    args: CommandArgs,
) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();
    let definition: String = args.req(0)?;
    let capella = args.get_flag("capella")?;
    let environment = args.get_flag("capella")?.unwrap_or("hosted".to_string());

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
        None => return Err(ShellError::unexpected("Could not auto-select a project")),
    };
    let project_id = find_project_id(ctrl_c.clone(), project_name, &client, deadline)?;

    if environment == "hosted".to_string() {
        let mut json: JSONCloudCreateClusterRequestV3 =
            serde_json::from_str(definition.as_str())
                .map_err(|e| ShellError::unexpected(e.to_string()))?;
        json.set_project_id(project_id);

        let response = client.capella_request(
            CapellaRequest::CreateClusterV3 {
                payload: serde_json::to_string(&json)?,
            },
            Instant::now().add(control.timeout()),
            ctrl_c,
        )?;
        if response.status() != 202 {
            return Err(ShellError::unexpected(response.content().to_string()));
        };

        return Ok(OutputStream::empty());
    }

    let cloud_name = match control.active_cloud() {
        Some(p) => p,
        None => return Err(ShellError::unexpected("Could not auto-select a cloud")),
    };
    let cloud_id = find_cloud_id(ctrl_c.clone(), cloud_name, &client, deadline)?;

    let mut json: JSONCloudCreateClusterRequest = serde_json::from_str(definition.as_str())
        .map_err(|e| ShellError::unexpected(e.to_string()))?;
    json.set_cloud_id(cloud_id);
    json.set_project_id(project_id);

    let response = client.capella_request(
        CapellaRequest::CreateCluster {
            payload: serde_json::to_string(&json)?,
        },
        Instant::now().add(control.timeout()),
        ctrl_c,
    )?;
    if response.status() != 202 {
        return Err(ShellError::unexpected(response.content().to_string()));
    };

    Ok(OutputStream::empty())
}
