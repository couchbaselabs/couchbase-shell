use crate::cli::cloud_json::JSONCloudDeleteAllowListRequest;
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

pub struct AddressesDrop {
    state: Arc<Mutex<State>>,
}

impl AddressesDrop {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for AddressesDrop {
    fn name(&self) -> &str {
        "addresses drop"
    }

    fn signature(&self) -> Signature {
        Signature::build("addresses drop").required_named(
            "address",
            SyntaxShape::String,
            "the address to add to allow access",
            None,
        )
    }

    fn usage(&self) -> &str {
        "Removes an address to disallow cloud cluster access"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        addresses_drop(self.state.clone(), args)
    }
}

fn addresses_drop(state: Arc<Mutex<State>>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();
    let address = args.req_named("address")?;

    debug!("Running address drop for {}", &address);

    let guard = state.lock().unwrap();
    let active_cluster = guard.active_cluster();

    if active_cluster.cloud().is_none() {
        return Err(ShellError::unexpected(
            "addresses add can only be used with clusters registered to a cloud control pane",
        ));
    }

    let identifier = guard.active();
    let cloud = guard
        .cloud_for_cluster(active_cluster.cloud().unwrap())?
        .cloud();
    let cluster_id = cloud.find_cluster_id(
        identifier,
        Instant::now().add(active_cluster.timeouts().query_timeout()),
        ctrl_c.clone(),
    )?;

    let entry = JSONCloudDeleteAllowListRequest::new(address);

    let response = cloud.cloud_request(
        CloudRequest::DeleteAllowListEntry {
            cluster_id,
            payload: serde_json::to_string(&entry)?,
        },
        Instant::now().add(active_cluster.timeouts().query_timeout()),
        ctrl_c,
    )?;

    match response.status() {
        204 => Ok(OutputStream::empty()),
        _ => Err(ShellError::untagged_runtime_error(
            response.content().to_string(),
        )),
    }
}
