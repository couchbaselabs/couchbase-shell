use crate::cli::util::find_cloud_cluster_id;
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

pub struct CloudsClustersDrop {
    state: Arc<Mutex<State>>,
}

impl CloudsClustersDrop {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for CloudsClustersDrop {
    fn name(&self) -> &str {
        "clouds clusters-drop"
    }

    fn signature(&self) -> Signature {
        Signature::build("clouds clusters-drop").required(
            "name",
            SyntaxShape::String,
            "the name of the cluster",
        )
    }

    fn usage(&self) -> &str {
        "Deletes a cloud cluster"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        clusters_drop(self.state.clone(), args)
    }
}

fn clusters_drop(state: Arc<Mutex<State>>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();
    let name: String = args.req(0)?;

    debug!("Running clouds clusters drop for {}", &name);

    let guard = state.lock().unwrap();
    let control = guard.active_cloud_control_plane()?;
    let client = control.client();

    let deadline = Instant::now().add(control.timeout());
    let cluster_id = find_cloud_cluster_id(ctrl_c.clone(), name, &client, deadline)?;
    let response =
        client.cloud_request(CloudRequest::DeleteCluster { cluster_id }, deadline, ctrl_c)?;
    if response.status() != 202 {
        return Err(ShellError::untagged_runtime_error(
            response.content().to_string(),
        ));
    };

    Ok(OutputStream::empty())
}
