//! The `buckets get` command fetches buckets from the server.

use crate::state::State;

use crate::cli::cloud_json::JSONCloudDeleteBucketRequest;
use crate::cli::util::{arg_as, cluster_identifiers_from};
use crate::client::{CloudRequest, HttpResponse, ManagementRequest};
use async_trait::async_trait;
use log::debug;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_stream::OutputStream;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

pub struct BucketsDrop {
    state: Arc<Mutex<State>>,
}

impl BucketsDrop {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for BucketsDrop {
    fn name(&self) -> &str {
        "buckets drop"
    }

    fn signature(&self) -> Signature {
        Signature::build("buckets drop")
            .required_named("name", SyntaxShape::String, "the name of the bucket", None)
            .named(
                "clusters",
                SyntaxShape::String,
                "the clusters which should be contacted",
                None,
            )
    }

    fn usage(&self) -> &str {
        "Drops buckets through the HTTP API"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        buckets_drop(self.state.clone(), args)
    }
}

fn buckets_drop(state: Arc<Mutex<State>>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();
    let args = args.evaluate_once()?;

    let cluster_identifiers = cluster_identifiers_from(&state, &args, true)?;
    let name = arg_as(&args, "name", |v| v.as_string())?.unwrap();

    debug!("Running buckets drop for bucket {:?}", &name);

    for identifier in cluster_identifiers {
        let guard = state.lock().unwrap();
        let cluster = match guard.clusters().get(&identifier) {
            Some(c) => c,
            None => {
                return Err(ShellError::untagged_runtime_error("Cluster not found"));
            }
        };

        let result: HttpResponse;
        if let Some(c) = cluster.cloud() {
            let identifier = guard.active();
            let cloud = guard.cloud_for_cluster(c)?.cloud();
            let cluster_id = cloud.find_cluster_id(
                identifier,
                Instant::now().add(cluster.timeouts().query_timeout()),
                ctrl_c.clone(),
            )?;
            let req = JSONCloudDeleteBucketRequest::new(name.clone());
            let payload = serde_json::to_string(&req)?;
            result = cloud.cloud_request(
                CloudRequest::DeleteBucket {
                    cluster_id,
                    payload,
                },
                Instant::now().add(cluster.timeouts().query_timeout()),
                ctrl_c.clone(),
            )?;
        } else {
            result = cluster.cluster().management_request(
                ManagementRequest::DropBucket { name: name.clone() },
                Instant::now().add(cluster.timeouts().query_timeout()),
                ctrl_c.clone(),
            )?;
        }

        match result.status() {
            200 => {}
            202 => {}
            _ => {
                return Err(ShellError::untagged_runtime_error(
                    result.content().to_string(),
                ))
            }
        }
    }

    Ok(OutputStream::empty())
}
