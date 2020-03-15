use crate::state::State;
use futures::executor::block_on;
use nu_cli::{CommandArgs, CommandRegistry, OutputStream};
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue};
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
        Signature::build("clusters").named(
            "activate",
            SyntaxShape::String,
            "the id of the cluster to activate",
            None,
        )
    }

    fn usage(&self) -> &str {
        "Lists all managed clusters"
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        block_on(clusters(args, registry, self.state.clone()))
    }
}

async fn clusters(
    args: CommandArgs,
    registry: &CommandRegistry,
    state: Arc<State>,
) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;

    if let Some(id) = args.get("activate") {
        state.set_active(id.as_string().unwrap()).unwrap();
    }

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
