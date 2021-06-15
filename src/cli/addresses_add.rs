use crate::cli::cloud_json::JSONCloudAppendAllowListRequest;
use crate::cli::util::arg_as;
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

pub struct AddressesAdd {
    state: Arc<Mutex<State>>,
}

impl AddressesAdd {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for AddressesAdd {
    fn name(&self) -> &str {
        "addresses add"
    }

    fn signature(&self) -> Signature {
        Signature::build("addresses add")
            .required_named("address", SyntaxShape::String, "the address to add to allow access", None)
            .named(
                "duration",
                SyntaxShape::String,
                "the duration (hours) to allow access from this address, if not set then address is added for permanent access",
                None,
            )
    }

    fn usage(&self) -> &str {
        "Adds an address to allow for cloud cluster access"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        addresses_add(self.state.clone(), args)
    }
}

fn addresses_add(state: Arc<Mutex<State>>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();
    let args = args.evaluate_once()?;
    let address = arg_as(&args, "address", |v| v.as_string())?.expect("address is required");
    let duration = arg_as(&args, "durability", |v| v.as_string())?;

    debug!("Running address add for {}", &address);

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

    let rule_type = if duration.is_some() {
        "temporary"
    } else {
        "permanent"
    };

    let entry = JSONCloudAppendAllowListRequest::new(address, rule_type.to_string(), duration);
    let response = cloud.cloud_request(
        CloudRequest::CreateAllowListEntry {
            cluster_id,
            payload: serde_json::to_string(&entry)?,
        },
        Instant::now().add(active_cluster.timeouts().query_timeout()),
        ctrl_c.clone(),
    )?;

    match response.status() {
        202 => Ok(OutputStream::empty()),
        _ => Err(ShellError::untagged_runtime_error(
            response.content().to_string(),
        )),
    }
}
