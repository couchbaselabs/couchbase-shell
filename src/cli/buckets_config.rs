use crate::cli::util::convert_json_value_to_nu_value;
use crate::client::ManagementRequest;
use crate::state::State;
use async_trait::async_trait;
use futures::executor::block_on;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_source::Tag;
use nu_stream::OutputStream;
use std::sync::Arc;

pub struct BucketsConfig {
    state: Arc<State>,
}

impl BucketsConfig {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for BucketsConfig {
    fn name(&self) -> &str {
        "buckets config"
    }

    fn signature(&self) -> Signature {
        Signature::build("buckets config").required(
            "name",
            SyntaxShape::String,
            "the name of the bucket",
        )
    }

    fn usage(&self) -> &str {
        "Shows the bucket config (low level)"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        buckets(args, self.state.clone())
    }
}

fn buckets(args: CommandArgs, state: Arc<State>) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once()?;

    let bucket_name = match args.nth(0) {
        Some(n) => n.as_string()?,
        None => {
            return Err(ShellError::untagged_runtime_error(format!(
                "No bucket name was specified"
            )))
        }
    };

    let cluster = match state.clusters().get(&state.active()) {
        Some(c) => c.cluster(),
        None => {
            return Err(ShellError::untagged_runtime_error("Cluster not found"));
        }
    };

    let response =
        block_on(cluster.management_request(ManagementRequest::GetBucket { name: bucket_name }));

    let content = serde_json::from_str(response.content()).unwrap();
    let converted = convert_json_value_to_nu_value(&content, Tag::default())?;

    Ok(vec![converted].into())
}
