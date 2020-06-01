use crate::state::State;
use nu_cli::{CommandArgs, CommandRegistry, OutputStream};
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, TaggedDictBuilder};
use nu_source::Tag;
use std::sync::Arc;
use async_trait::async_trait;

pub struct UseCluster {
    state: Arc<State>,
}

impl UseCluster {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_cli::WholeStreamCommand for UseCluster {
    fn name(&self) -> &str {
        "use cluster"
    }

    fn signature(&self) -> Signature {
        Signature::build("use cluster").required(
            "identifier",
            SyntaxShape::String,
            "the identifier of the cluster",
        )
    }

    fn usage(&self) -> &str {
        "Sets the active cluster based on its identifier"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        use_cmd(args, registry, self.state.clone()).await
    }
}

async fn use_cmd(
    args: CommandArgs,
    registry: &CommandRegistry,
    state: Arc<State>,
) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry).await?;

    if let Some(id) = args.nth(0) {
        state.set_active(id.as_string().unwrap()).unwrap();
    }

    let mut using_now = TaggedDictBuilder::new(Tag::default());
    using_now.insert_value("cluster", state.active());
    let clusters = vec![using_now.into_value()];
    Ok(clusters.into())
}
