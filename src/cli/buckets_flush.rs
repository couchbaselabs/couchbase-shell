//! The `buckets get` command fetches buckets from the server.

use crate::state::State;

use crate::cli::util::{cluster_identifiers_from, validate_is_not_cloud};
use crate::client::ManagementRequest;
use async_trait::async_trait;
use log::debug;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_stream::OutputStream;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

pub struct BucketsFlush {
    state: Arc<Mutex<State>>,
}

impl BucketsFlush {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for BucketsFlush {
    fn name(&self) -> &str {
        "buckets flush"
    }

    fn signature(&self) -> Signature {
        Signature::build("buckets flush")
            .required("name", SyntaxShape::String, "the name of the bucket")
            .named(
                "clusters",
                SyntaxShape::String,
                "the clusters which should be contacted",
                None,
            )
    }

    fn usage(&self) -> &str {
        "Flushes buckets through the HTTP API"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        buckets_flush(self.state.clone(), args)
    }
}

fn buckets_flush(state: Arc<Mutex<State>>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();

    let cluster_identifiers = cluster_identifiers_from(&state, &args, true)?;
    let name: String = args.req(0)?;
    let bucket: String = args.get_flag("bucket")?.unwrap_or_else(|| "".into());

    debug!("Running buckets flush for bucket {:?}", &bucket);

    for identifier in cluster_identifiers {
        let guard = state.lock().unwrap();
        let cluster = match guard.clusters().get(&identifier) {
            Some(c) => c,
            None => {
                return Err(ShellError::unexpected("Cluster not found"));
            }
        };
        validate_is_not_cloud(
            cluster,
            "buckets flush cannot be run against Capella clusters",
        )?;

        let result = cluster.cluster().http_client().management_request(
            ManagementRequest::FlushBucket { name: name.clone() },
            Instant::now().add(cluster.timeouts().management_timeout()),
            ctrl_c.clone(),
        )?;

        match result.status() {
            200 => {}
            _ => {
                return Err(ShellError::unexpected(result.content()));
            }
        }
    }

    Ok(OutputStream::empty())
}
