use crate::cli::buckets_builder::{BucketSettings, JSONBucketSettings, JSONCloudBucketSettings};
use crate::cli::buckets_get::bucket_to_tagged_dict;
use crate::cli::util::cluster_identifiers_from;
use crate::client::{CloudRequest, ManagementRequest};
use crate::state::State;
use log::debug;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_stream::OutputStream;
use std::convert::TryFrom;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

pub struct Buckets {
    state: Arc<Mutex<State>>,
}

impl Buckets {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl nu_engine::WholeStreamCommand for Buckets {
    fn name(&self) -> &str {
        "buckets"
    }

    fn signature(&self) -> Signature {
        Signature::build("buckets").named(
            "clusters",
            SyntaxShape::String,
            "the clusters which should be contacted",
            None,
        )
    }

    fn usage(&self) -> &str {
        "Perform bucket management operations"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        buckets_get_all(self.state.clone(), args)
    }
}

fn buckets_get_all(
    state: Arc<Mutex<State>>,
    args: CommandArgs,
) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();

    let cluster_identifiers = cluster_identifiers_from(&state, &args, true)?;

    debug!("Running buckets");

    let guard = state.lock().unwrap();
    let mut results = vec![];
    for identifier in cluster_identifiers {
        let cluster = match guard.clusters().get(&identifier) {
            Some(c) => c,
            None => {
                return Err(ShellError::untagged_runtime_error("Cluster not found"));
            }
        };

        if let Some(plane) = cluster.cloud_org() {
            let cloud = guard.cloud_org_for_cluster(plane)?.client();
            let deadline = Instant::now().add(cluster.timeouts().management_timeout());
            let cluster_id = cloud.find_cluster_id(identifier.clone(), deadline, ctrl_c.clone())?;
            let response = cloud.cloud_request(
                CloudRequest::GetBuckets { cluster_id },
                deadline,
                ctrl_c.clone(),
            )?;
            if response.status() != 200 {
                return Err(ShellError::unexpected(response.content()));
            }

            let content: Vec<JSONCloudBucketSettings> = serde_json::from_str(response.content())?;
            for bucket in content.into_iter() {
                results.push(bucket_to_tagged_dict(
                    BucketSettings::try_from(bucket)?,
                    identifier.clone(),
                    true,
                ));
            }
        } else {
            let response = cluster.cluster().http_client().management_request(
                ManagementRequest::GetBuckets,
                Instant::now().add(cluster.timeouts().management_timeout()),
                ctrl_c.clone(),
            )?;

            let content: Vec<JSONBucketSettings> = serde_json::from_str(response.content())?;

            for bucket in content.into_iter() {
                results.push(bucket_to_tagged_dict(
                    BucketSettings::try_from(bucket)?,
                    identifier.clone(),
                    false,
                ));
            }
        }
    }

    Ok(OutputStream::from(results))
}
