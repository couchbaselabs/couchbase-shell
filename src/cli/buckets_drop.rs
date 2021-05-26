//! The `buckets get` command fetches buckets from the server.

use crate::state::State;

use crate::cli::util::cluster_identifiers_from;
use crate::client::ManagementRequest;
use async_trait::async_trait;
use log::debug;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_stream::OutputStream;
use std::ops::Add;
use std::sync::Arc;
use tokio::time::Instant;

pub struct BucketsDrop {
    state: Arc<State>,
}

impl BucketsDrop {
    pub fn new(state: Arc<State>) -> Self {
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

fn buckets_drop(state: Arc<State>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();
    let args = args.evaluate_once()?;

    let cluster_identifiers = cluster_identifiers_from(&state, &args, true)?;
    let name = match args.call_info.args.get("name") {
        Some(v) => match v.as_string() {
            Ok(name) => name,
            Err(e) => return Err(e),
        },
        None => return Err(ShellError::unexpected("name is required")),
    };
    let bucket = match args
        .call_info
        .args
        .get("bucket")
        .map(|bucket| bucket.as_string().ok())
        .flatten()
    {
        Some(v) => v,
        None => "".into(),
    };

    debug!("Running buckets drop for bucket {:?}", &bucket);

    for identifier in cluster_identifiers {
        let cluster = match state.clusters().get(&identifier) {
            Some(c) => c,
            None => {
                return Err(ShellError::untagged_runtime_error("Cluster not found"));
            }
        };

        let result = cluster.cluster().management_request(
            ManagementRequest::DropBucket { name: name.clone() },
            Instant::now().add(cluster.timeouts().query_timeout()),
            ctrl_c.clone(),
        )?;

        match result.status() {
            200 => {}
            _ => {
                return Err(ShellError::untagged_runtime_error(
                    result.content().to_string(),
                ))
            }
        }
    }

    Ok(OutputStream::empty())
}
