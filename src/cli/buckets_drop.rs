//! The `buckets get` command fetches buckets from the server.

use crate::state::State;

use crate::cli::buckets_create::collected_value_from_error_string;
use crate::cli::cloud_json::JSONCloudDeleteBucketRequest;
use crate::cli::util::cluster_identifiers_from;
use crate::client::{CloudRequest, HttpResponse, ManagementRequest};
use async_trait::async_trait;
use log::debug;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, Value};
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
            .required("name", SyntaxShape::String, "the name of the bucket")
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

    let cluster_identifiers = cluster_identifiers_from(&state, &args, true)?;
    let name: String = args.req(0)?;
    let guard = state.lock().unwrap();

    debug!("Running buckets drop for bucket {:?}", &name);

    let mut results: Vec<Value> = vec![];
    for identifier in cluster_identifiers {
        let cluster = match guard.clusters().get(&identifier) {
            Some(c) => c,
            None => {
                results.push(collected_value_from_error_string(
                    identifier.clone(),
                    "Cluster not found",
                ));
                continue;
            }
        };

        let result: HttpResponse;
        if let Some(plane) = cluster.cloud_org() {
            let cloud = guard.cloud_org_for_cluster(plane)?.client();
            let deadline = Instant::now().add(cluster.timeouts().management_timeout());
            let cluster_id =
                cloud.find_cluster_id(identifier.clone(), deadline.clone(), ctrl_c.clone())?;
            let req = JSONCloudDeleteBucketRequest::new(name.clone());
            let payload = serde_json::to_string(&req)?;
            result = cloud.cloud_request(
                CloudRequest::DeleteBucket {
                    cluster_id,
                    payload,
                },
                deadline,
                ctrl_c.clone(),
            )?;
        } else {
            result = cluster.cluster().http_client().management_request(
                ManagementRequest::DropBucket { name: name.clone() },
                Instant::now().add(cluster.timeouts().management_timeout()),
                ctrl_c.clone(),
            )?;
        }

        match result.status() {
            200 => {}
            202 => {}
            _ => {
                results.push(collected_value_from_error_string(
                    identifier.clone(),
                    result.content(),
                ));
            }
        }
    }

    Ok(OutputStream::from(results))
}
