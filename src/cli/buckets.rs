use crate::state::State;
use futures::executor::block_on;
use nu::{CommandArgs, CommandRegistry, OutputStream};
use nu_errors::ShellError;
use nu_protocol::{Signature, TaggedDictBuilder};
use nu_source::Tag;
use serde::Deserialize;
use std::sync::Arc;

pub struct Buckets {
    state: Arc<State>,
}

impl Buckets {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

impl nu::WholeStreamCommand for Buckets {
    fn name(&self) -> &str {
        "buckets"
    }

    fn signature(&self) -> Signature {
        Signature::build("buckets")
    }

    fn usage(&self) -> &str {
        "Lists all buckets of the connected cluster"
    }

    fn run(
        &self,
        _args: CommandArgs,
        _registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        block_on(buckets(self.state.clone()))
    }
}

async fn buckets(state: Arc<State>) -> Result<OutputStream, ShellError> {
    let client = reqwest::Client::new();

    // todo: hack! need to actually use proper hostname from a parsed connstr...
    let host = state.connstr().replace("couchbase://", "");
    let uri = format!("http://{}:8091/pools/default/buckets", host);

    let resp = client
        .get(&uri)
        .basic_auth(state.username(), Some(state.password()))
        .send()
        .await
        .unwrap()
        .json::<Vec<BucketInfo>>()
        .await
        .unwrap();

    let buckets = resp
        .into_iter()
        .map(|n| {
            let mut collected = TaggedDictBuilder::new(Tag::default());
            collected.insert_value("name", n.name);
            collected.insert_value("type", n.bucket_type);
            collected.into_value()
        })
        .collect::<Vec<_>>();

    Ok(buckets.into())
}

#[derive(Debug, Deserialize)]
struct BucketInfo {
    name: String,
    #[serde(rename = "bucketType")]
    bucket_type: String,
}
