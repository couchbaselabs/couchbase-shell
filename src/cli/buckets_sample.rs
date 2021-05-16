use crate::cli::util::cluster_identifiers_from;
use crate::client::ManagementRequest;
use crate::state::State;
use async_trait::async_trait;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, TaggedDictBuilder, Value};
use nu_source::Tag;
use nu_stream::OutputStream;
use std::ops::Add;
use std::sync::Arc;
use tokio::time::Instant;

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

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        load_sample_bucket(self.state.clone(), args)
    }
}

fn load_sample_bucket(state: Arc<State>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once()?;

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
            Some(c) => c,
            None => {
                return Err(ShellError::untagged_runtime_error("Cluster not found"));
            }
        };

        let response = cluster.cluster().management_request(
            ManagementRequest::LoadSampleBucket {
                name: format!("[\"{}\"]", bucket_name),
            },
            Instant::now().add(cluster.timeouts().query_timeout()),
        )?;

        match response.status() {
            202 => {}
            _ => {
                return Err(ShellError::untagged_runtime_error(format!(
                    "{}",
                    response.content()
                )))
            }
        }

        let resp: Vec<String> = serde_json::from_str(response.content())?;
        for r in resp {
            let mut collected = TaggedDictBuilder::new(Tag::default());
            collected.insert_value("cluster", identifier.clone());
            collected.insert_value("results", r);
            results.push(collected.into_value());
        }
    }

    Ok(OutputStream::from(results))
}
