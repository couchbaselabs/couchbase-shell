//! The `buckets get` command fetches buckets from the server.

use crate::state::State;
use couchbase::FlushBucketOptions;

use crate::cli::util::cluster_identifiers_from;
use async_trait::async_trait;
use log::debug;
use nu_cli::{CommandArgs, OutputStream};
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use std::sync::Arc;

pub struct BucketsFlush {
    state: Arc<State>,
}

impl BucketsFlush {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_cli::WholeStreamCommand for BucketsFlush {
    fn name(&self) -> &str {
        "buckets flush"
    }

    fn signature(&self) -> Signature {
        Signature::build("buckets flush")
            .required_named("name", SyntaxShape::String, "the name of the bucket", None)
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

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        buckets_flush(self.state.clone(), args).await
    }
}

async fn buckets_flush(state: Arc<State>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once().await?;

    let cluster_identifiers = cluster_identifiers_from(&state, &args, true)?;
    let name = match args.get("name") {
        Some(v) => match v.as_string() {
            Ok(name) => name,
            Err(e) => return Err(e),
        },
        None => return Err(ShellError::unexpected("name is required")),
    };
    let bucket = match args
        .get("bucket")
        .map(|bucket| bucket.as_string().ok())
        .flatten()
    {
        Some(v) => v,
        None => "".into(),
    };

    debug!("Running buckets flush for bucket {:?}", &bucket);

    for identifier in cluster_identifiers {
        let cluster = match state.clusters().get(&identifier) {
            Some(c) => c.cluster(),
            None => {
                return Err(ShellError::untagged_runtime_error("Cluster not found"));
            }
        };

        let mgr = cluster.buckets();
        let result = mgr
            .flush_bucket(name.clone(), FlushBucketOptions::default())
            .await;

        match result {
            Ok(_) => {}
            Err(e) => return Err(ShellError::untagged_runtime_error(format!("{}", e))),
        }
    }

    Ok(OutputStream::empty())
}
