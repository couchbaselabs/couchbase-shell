use crate::cli::cloud_json::JSONCloudCreateClusterRequest;
use crate::cli::util::{find_cloud_id, find_project_id};
use crate::client::CloudRequest;
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

pub struct CloudsClustersCreate {
    state: Arc<Mutex<State>>,
}

impl CloudsClustersCreate {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for CloudsClustersCreate {
    fn name(&self) -> &str {
        "clouds clusters-create"
    }

    fn signature(&self) -> Signature {
        Signature::build("clouds clusters-create").required(
            "definition",
            SyntaxShape::String,
            "the definition of the cluster",
        )
    }

    fn usage(&self) -> &str {
        "Creates a new cloud cluster"
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

    debug!("Running clouds clusters create for {}", &definition);

    let guard = state.lock().unwrap();
    let control = guard.active_cloud_org()?;
    let client = control.client();

    let deadline = Instant::now().add(control.timeout());
    let cloud = guard.active_cloud()?;
    let cloud_name = guard.active_cloud_name().unwrap();
    let project_name = match cloud.active_project() {
        Some(p) => p,
        None => return Err(ShellError::unexpected("Could not auto-select a project")),
    };
    let cloud_id = find_cloud_id(ctrl_c.clone(), cloud_name, &client, deadline)?;
    let project_id = find_project_id(ctrl_c.clone(), project_name, &client, deadline)?;

    let mut json: JSONCloudCreateClusterRequest = serde_json::from_str(definition.as_str())
        .map_err(|e| ShellError::unexpected(e.to_string()))?;
    json.set_cloud_id(cloud_id);
    json.set_project_id(project_id);

    let response = client.cloud_request(
        CloudRequest::CreateCluster {
            payload: serde_json::to_string(&json)?,
        },
        Instant::now().add(control.timeout()),
        ctrl_c,
    )?;
    if response.status() != 202 {
        return Err(ShellError::untagged_runtime_error(
            response.content().to_string(),
        ));
    };

    Ok(OutputStream::empty())
}
