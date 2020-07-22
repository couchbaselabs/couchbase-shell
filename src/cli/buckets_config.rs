use crate::cli::convert_cb_error;
use crate::cli::util::convert_json_value_to_nu_value;
use crate::state::State;
use async_trait::async_trait;
use couchbase::{GenericManagementRequest, Request};
use futures::channel::oneshot;
use nu_cli::{CommandArgs, CommandRegistry, OutputStream};
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_source::Tag;
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
impl nu_cli::WholeStreamCommand for BucketsConfig {
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

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        buckets(args, registry, self.state.clone()).await
    }
}

async fn buckets(
    args: CommandArgs,
    registry: &CommandRegistry,
    state: Arc<State>,
) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry).await?;

    let bucket_name = match args.nth(0) {
        Some(n) => n.as_string()?,
        None => {
            return Err(ShellError::untagged_runtime_error(format!(
                "No bucket name was specified"
            )))
        }
    };

    let core = state.active_cluster().cluster().core();

    let (sender, receiver) = oneshot::channel();
    let request = GenericManagementRequest::new(
        sender,
        format!("/pools/default/buckets/{}", &bucket_name),
        "get".into(),
        None,
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
                "Empty response from cluster even though got 200 ok",
            ));
        }
    };
    let resp = serde_json::from_slice(payload)?;
    let converted = convert_json_value_to_nu_value(&resp, Tag::default());

    Ok(vec![converted].into())
}
