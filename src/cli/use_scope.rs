use crate::state::State;
use async_trait::async_trait;
use nu_cli::{CommandArgs, CommandRegistry, OutputStream};
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, TaggedDictBuilder};
use nu_source::Tag;
use std::sync::Arc;

pub struct UseScope {
    state: Arc<State>,
}

impl UseScope {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_cli::WholeStreamCommand for UseScope {
    fn name(&self) -> &str {
        "use scope"
    }

    fn signature(&self) -> Signature {
        Signature::build("use scope").required(
            "identifier",
            SyntaxShape::String,
            "the name of the scope",
        )
    }

    fn usage(&self) -> &str {
        "Sets the active scope based on its name"
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

    if active.active_bucket().is_none() {
        return Err(ShellError::untagged_runtime_error(
            "You must select a bucket before a scope",
        ));
    }

    if let Some(id) = args.nth(0) {
        active.set_active_scope(id.as_string()?);
    }

    let mut using_now = TaggedDictBuilder::new(Tag::default());
    using_now.insert_value(
        "scope",
        active
            .active_scope()
            .map(|s| s.clone())
            .unwrap_or(String::from("<not set>")),
    );
    let clusters = vec![using_now.into_value()];
    Ok(clusters.into())
}
