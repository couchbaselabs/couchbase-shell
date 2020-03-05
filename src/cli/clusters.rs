use crate::state::State;
use futures::executor::block_on;
use nu_cli::{CommandArgs, CommandRegistry, OutputStream};
use nu_errors::ShellError;
use nu_protocol::{Signature, TaggedDictBuilder, UntaggedValue};
use nu_source::Tag;
use serde::Deserialize;
use std::sync::Arc;

pub struct Clusters {
    state: Arc<State>,
}

impl Clusters {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

impl nu_cli::WholeStreamCommand for Clusters {
    fn name(&self) -> &str {
        "clusters"
    }

    fn signature(&self) -> Signature {
        Signature::build("clusters")
    }

    fn usage(&self) -> &str {
        "Lists all managed clusters"
    }

    fn run(
        &self,
        _args: CommandArgs,
        _registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        block_on(clusters(self.state.clone()))
    }
}

async fn clusters(state: Arc<State>) -> Result<OutputStream, ShellError> {
    let active = state.active();
    let clusters = state
        .clusters()
        .iter()
        .map(|(k, v)| {
            let mut collected = TaggedDictBuilder::new(Tag::default());
            collected.insert_untagged("active", UntaggedValue::boolean(k == &active));
            collected.insert_value("identifier", k.clone());
            collected.insert_value("connstr", String::from(v.connstr()));
            collected.insert_value("username", String::from(v.username()));
            collected.into_value()
        })
        .collect::<Vec<_>>();

    Ok(clusters.into())
}

#[derive(Debug, Deserialize)]
struct BucketInfo {
    name: String,
    #[serde(rename = "bucketType")]
    bucket_type: String,
}
