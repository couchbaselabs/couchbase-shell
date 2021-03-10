use crate::cli::convert_cb_error;
use crate::cli::util::cluster_identifiers_from;
use crate::state::State;
use async_trait::async_trait;
use couchbase::{GenericManagementRequest, Request};
use futures::channel::oneshot;
use nu_cli::OutputStream;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, TaggedDictBuilder, Value};
use nu_source::Tag;
use std::sync::Arc;

pub struct BucketsSample {
    state: Arc<State>,
}

impl BucketsSample {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for BucketsSample {
    fn name(&self) -> &str {
        "buckets load-sample"
    }

    fn signature(&self) -> Signature {
        Signature::build("buckets load-sample")
            .required(
                "name",
                SyntaxShape::String,
                "the name of the bucket to load",
            )
            .named(
                "clusters",
                SyntaxShape::String,
                "the clusters which should be contacted",
                None,
            )
    }

    fn usage(&self) -> &str {
        "Load a sample bucket"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        load_sample_bucket(self.state.clone(), args).await
    }
}

async fn load_sample_bucket(
    state: Arc<State>,
    args: CommandArgs,
) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once().await?;

    let cluster_identifiers = cluster_identifiers_from(&state, &args, true)?;
    let bucket_name = match args.nth(0) {
        Some(n) => n.as_string()?,
        None => {
            return Err(ShellError::untagged_runtime_error(format!(
                "No bucket name was specified"
            )))
        }
    };

    let mut results: Vec<Value> = vec![];
    for identifier in cluster_identifiers {
        let cluster = match state.clusters().get(&identifier) {
            Some(c) => c.cluster(),
            None => {
                return Err(ShellError::untagged_runtime_error("Cluster not found"));
            }
        };

        let core = cluster.core();

        let (sender, receiver) = oneshot::channel();
        let request = GenericManagementRequest::new(
            sender,
            "/sampleBuckets/install".into(),
            "post".into(),
            Some(format!("[\"{}\"]", bucket_name)),
        );
        core.send(Request::GenericManagementRequest(request));

        let input = match receiver.await {
            Ok(i) => i,
            Err(e) => {
                return Err(ShellError::untagged_runtime_error(format!(
                    "Error streaming result {}",
                    e
                )))
            }
        };
        let result = convert_cb_error(input)?;

        let payload = match result.payload() {
            Some(p) => p,
            None => {
                return Err(ShellError::untagged_runtime_error(
                    "Empty response from cluster even though got ok",
                ));
            }
        };

        let resp: Vec<String> = serde_json::from_slice(payload)?;
        for r in resp {
            let mut collected = TaggedDictBuilder::new(Tag::default());
            collected.insert_value("cluster", identifier.clone());
            collected.insert_value("results", r);
            results.push(collected.into_value());
        }
    }

    Ok(OutputStream::from(results))
}
