use crate::cli::convert_cb_error;
use crate::cli::util::cluster_identifiers_from;
use crate::state::State;
use async_trait::async_trait;
use couchbase::{KvStatsRequest, Request};
use futures::channel::oneshot;
use futures::stream::StreamExt;
use nu_cli::{CommandArgs, CommandRegistry, OutputStream};
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, TaggedDictBuilder};
use nu_source::Tag;
use std::sync::Arc;

pub struct DataStats {
    state: Arc<State>,
}

impl DataStats {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_cli::WholeStreamCommand for DataStats {
    fn name(&self) -> &str {
        "data stats"
    }

    fn signature(&self) -> Signature {
        Signature::build("data stats").named(
            "clusters",
            SyntaxShape::String,
            "the clusters which should be contacted",
            None,
        )
    }

    fn usage(&self) -> &str {
        "Loads Key/Value statistics from the cluster"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        run_stats(self.state.clone(), args, registry).await
    }
}

async fn run_stats(
    state: Arc<State>,
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry).await?;

    let identifier_arg = args
        .get("clusters")
        .map(|id| id.as_string().unwrap())
        .unwrap_or_else(|| state.active());

    let cluster_identifiers = cluster_identifiers_from(&state, identifier_arg.as_str());

    let mut stats = vec![];

    for identifier in cluster_identifiers {
        let core = state.clusters().get(&identifier).unwrap().cluster().core();
        let (sender, receiver) = oneshot::channel();
        let request = KvStatsRequest::new(sender);
        core.send(Request::KvStatsRequest(request));

        let mut result = convert_cb_error(receiver.await.unwrap())?;
        let mut s = result
            .stats()
            .map(|stat| {
                let mut collected = TaggedDictBuilder::new(Tag::default());
                collected.insert_value("cluster", identifier.clone());

                let node = stat.server().split(':').nth(0).unwrap();
                collected.insert_value("node", String::from(node));

                collected.insert_value("key", String::from(stat.key()));
                collected.insert_value("value", String::from(stat.value()));
                collected.into_value()
            })
            .collect::<Vec<_>>()
            .await;

        stats.append(&mut s);
    }

    Ok(stats.into())
}
