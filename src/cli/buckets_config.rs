use crate::cli::util::convert_json_value_to_nu_value;
use crate::state::State;
use futures::executor::block_on;
use nu_cli::{CommandArgs, CommandRegistry, OutputStream};
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_source::Tag;
use serde_json::Value;
use std::sync::Arc;

pub struct BucketsConfig {
    state: Arc<State>,
}

impl BucketsConfig {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

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

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        block_on(buckets(args, registry, self.state.clone()))
    }
}

async fn buckets(
    args: CommandArgs,
    registry: &CommandRegistry,
    state: Arc<State>,
) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;

    let bucket_name = args.nth(0).unwrap().as_string().unwrap();

    let client = reqwest::Client::new();

    // todo: hack! need to actually use proper hostname from a parsed connstr...
    let host = state.active_cluster().connstr().replace("couchbase://", "");
    let uri = format!(
        "http://{}:8091/pools/default/buckets/{}",
        host, &bucket_name
    );

    let resp = client
        .get(&uri)
        .basic_auth(
            state.active_cluster().username(),
            Some(state.active_cluster().password()),
        )
        .send()
        .await
        .unwrap()
        .json::<Value>()
        .await
        .unwrap();

    let converted = convert_json_value_to_nu_value(&resp, Tag::default());

    Ok(vec![converted].into())
}
