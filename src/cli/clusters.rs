use crate::cli::util::cluster_identifiers_from;
use crate::state::State;
use async_trait::async_trait;
use nu_cli::{CommandArgs, CommandRegistry, OutputStream};
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue};
use nu_source::Tag;
use std::sync::Arc;

pub struct Clusters {
    state: Arc<State>,
}

impl Clusters {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_cli::WholeStreamCommand for Clusters {
    fn name(&self) -> &str {
        "clusters"
    }

    fn signature(&self) -> Signature {
        Signature::build("clusters").named(
            "clusters",
            SyntaxShape::String,
            "the clusters which should be contacted",
            None,
        )
    }

    fn usage(&self) -> &str {
        "Lists all managed clusters"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        clusters(args, registry, self.state.clone()).await
    }
}

async fn clusters(
    args: CommandArgs,
    registry: &CommandRegistry,
    state: Arc<State>,
) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry).await?;

    let identifier_arg = args
        .get("clusters")
        .map(|id| id.as_string().unwrap())
        .unwrap_or_else(|| ".*".to_string());

    let identifiers = cluster_identifiers_from(&state, identifier_arg.as_str());

    let active = state.active();
    let clusters = state
        .clusters()
        .iter()
        .filter(|(k, _)| identifiers.contains(k))
        .map(|(k, v)| {
            let mut collected = TaggedDictBuilder::new(Tag::default());
            collected.insert_untagged("active", UntaggedValue::boolean(k == &active));
            collected.insert_value(
                "tls",
                UntaggedValue::boolean(v.connstr().starts_with("couchbases://")),
            );
            collected.insert_value("identifier", k.clone());
            collected.insert_value("username", String::from(v.username()));
            collected.into_value()
        })
        .collect::<Vec<_>>();

    Ok(clusters.into())
}
