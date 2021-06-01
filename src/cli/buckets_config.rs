use crate::cli::util::convert_json_value_to_nu_value;
use crate::client::ManagementRequest;
use crate::state::State;
use async_trait::async_trait;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_source::Tag;
use nu_stream::OutputStream;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

pub struct BucketsConfig {
    state: Arc<Mutex<State>>,
}

impl BucketsConfig {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
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

fn buckets(args: CommandArgs, state: Arc<Mutex<State>>) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();
    let args = args.evaluate_once()?;

    let bucket_name = match args.nth(0) {
        Some(n) => n.as_string()?,
        None => {
            return Err(ShellError::untagged_runtime_error(
                "No bucket name was specified".to_string(),
            ))
        }
    };

    let guard = state.lock().unwrap();
    let active_cluster = guard.active_cluster();
    let cluster = active_cluster.cluster();

    let response = cluster.management_request(
        ManagementRequest::GetBucket { name: bucket_name },
        Instant::now().add(active_cluster.timeouts().query_timeout()),
        ctrl_c,
    )?;

    let content = serde_json::from_str(response.content())?;
    let converted = convert_json_value_to_nu_value(&content, Tag::default())?;

    Ok(vec![converted].into())
}
