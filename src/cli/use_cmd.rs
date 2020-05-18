use crate::state::State;
use futures::executor::block_on;
use nu_cli::{CommandArgs, CommandRegistry, OutputStream};
use nu_errors::ShellError;
use nu_protocol::{Signature, TaggedDictBuilder};
use nu_source::Tag;
use std::sync::Arc;

pub struct UseCmd {
    state: Arc<State>,
}

impl UseCmd {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

impl nu_cli::WholeStreamCommand for UseCmd {
    fn name(&self) -> &str {
        "use"
    }

    fn signature(&self) -> Signature {
        Signature::build("use")
    }

    fn usage(&self) -> &str {
        "Modify the default execution environment of commands"
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        block_on(use_cmd(args, registry, self.state.clone()))
    }
}

async fn use_cmd(
    args: CommandArgs,
    registry: &CommandRegistry,
    state: Arc<State>,
) -> Result<OutputStream, ShellError> {
    let _args = args.evaluate_once(registry).await?;

    let active = state.active_cluster();
    let mut using_now = TaggedDictBuilder::new(Tag::default());
    using_now.insert_value("cluster", state.active());
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
