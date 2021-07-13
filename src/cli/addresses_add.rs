use crate::cli::buckets_create::collected_value_from_error_string;
use crate::cli::cloud_json::JSONCloudAppendAllowListRequest;
use crate::cli::util::{cluster_identifiers_from, validate_is_cloud};
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
            .required("address", SyntaxShape::String, "the address to add to allow access")
            .named(
                "duration",
                SyntaxShape::String,
                "the duration (hours) to allow access from this address, if not set then address is added for permanent access",
                None,
            )
            .named(
                "clusters",
                SyntaxShape::String,
                "the clusters which should be contacted",
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
    let address: String = args.req(0)?;
    let duration = args.get_flag("duration")?;

    debug!("Running address add for {}", &address);

    let cluster_identifiers = cluster_identifiers_from(&state, &args, true)?;
    let guard = state.lock().unwrap();

    let mut results = vec![];
    for identifier in cluster_identifiers {
        let active_cluster = match guard.clusters().get(&identifier) {
            Some(c) => c,
            None => {
                return Err(ShellError::untagged_runtime_error("Cluster not found"));
            }
        };
        validate_is_cloud(
            active_cluster,
            "addresses can only be used with clusters registered to a cloud control pane",
        )?;

        let deadline = Instant::now().add(active_cluster.timeouts().management_timeout());
        let cloud = guard
            .control_plane_for_cluster(active_cluster.cloud_control_plane().unwrap())?
            .client();
        let cluster_id =
            cloud.find_cluster_id(identifier.clone(), deadline.clone(), ctrl_c.clone())?;

        let rule_type = if duration.is_some() {
            "temporary"
        } else {
            "permanent"
        };

        let entry = JSONCloudAppendAllowListRequest::new(
            address.clone(),
            rule_type.to_string(),
            duration.clone(),
        );
        let response = cloud.cloud_request(
            CloudRequest::CreateAllowListEntry {
                cluster_id,
                payload: serde_json::to_string(&entry)?,
            },
            deadline,
            ctrl_c.clone(),
        )?;

        match response.status() {
            202 => {}
            _ => {
                results.push(collected_value_from_error_string(
                    identifier.clone(),
                    response.content(),
                ));
            }
        };
    }
    Ok(OutputStream::from(results))
}
