use crate::state::State;
use nu_cli::{CommandArgs, CommandRegistry, OutputStream};
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, TaggedDictBuilder};
use nu_source::Tag;
use std::sync::Arc;
use async_trait::async_trait;

pub struct UseBucket {
    state: Arc<State>,
}

impl UseBucket {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_cli::WholeStreamCommand for UseBucket {
    fn name(&self) -> &str {
        "use bucket"
    }

    fn signature(&self) -> Signature {
        Signature::build("use bucket").required(
            "identifier",
            SyntaxShape::String,
            "the name of the bucket",
        )
    }

    fn usage(&self) -> &str {
        "Sets the active bucket based on its name"
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

    let active = state.active_cluster();

    if let Some(id) = args.nth(0) {
        active.set_active_bucket(id.as_string().unwrap());
    }

    let mut using_now = TaggedDictBuilder::new(Tag::default());
    using_now.insert_value(
        "bucket",
        active
            .active_bucket()
            .map(|s| s.clone())
            .unwrap_or(String::from("<not set>")),
    );
    let clusters = vec![using_now.into_value()];
    Ok(clusters.into())
}
