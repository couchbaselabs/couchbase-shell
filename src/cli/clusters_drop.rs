use crate::cli::util::{find_capella_cluster_id_hosted, find_capella_cluster_id_vpc};
use crate::client::CapellaRequest;
use crate::state::{CapellaEnvironment, State};
use async_trait::async_trait;
use log::debug;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_stream::OutputStream;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

pub struct ClustersDrop {
    state: Arc<Mutex<State>>,
}

impl ClustersDrop {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for ClustersDrop {
    fn name(&self) -> &str {
        "clusters drop"
    }

    fn signature(&self) -> Signature {
        Signature::build("clusters drop")
            .required("name", SyntaxShape::String, "the name of the cluster")
            .named(
                "capella",
                SyntaxShape::String,
                "the Capella organization to use",
                None,
            )
    }

    fn usage(&self) -> &str {
        "Deletes a cluster from the active Capella organization"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        clusters_drop(self.state.clone(), args)
    }
}

fn clusters_drop(state: Arc<Mutex<State>>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();
    let name: String = args.req(0)?;
    let capella = args.get_flag("capella")?;

    debug!("Running clusters drop for {}", &name);

    let guard = state.lock().unwrap();
    let control = if let Some(c) = capella.clone() {
        guard.capella_org_for_cluster(c)
    } else {
        guard.active_capella_org()
    }?;

    let client = control.client();

    let deadline = Instant::now().add(control.timeout());
    let cluster_id = if control.environment() == CapellaEnvironment::Hosted {
        find_capella_cluster_id_hosted(ctrl_c.clone(), name, &client, deadline)
    } else {
        find_capella_cluster_id_vpc(ctrl_c.clone(), name, &client, deadline)
    }?;
    let response = if control.environment() == CapellaEnvironment::Hosted {
        client.capella_request(
            CapellaRequest::DeleteClusterV3 { cluster_id },
            deadline,
            ctrl_c,
        )
    } else {
        client.capella_request(
            CapellaRequest::DeleteCluster { cluster_id },
            deadline,
            ctrl_c,
        )
    }?;
    if response.status() != 202 {
        return Err(ShellError::unexpected(response.content().to_string()));
    };

    Ok(OutputStream::empty())
}
