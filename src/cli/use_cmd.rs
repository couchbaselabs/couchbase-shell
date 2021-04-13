use crate::state::State;
use async_trait::async_trait;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, TaggedDictBuilder};
use nu_source::Tag;
use nu_stream::OutputStream;
use std::sync::Arc;

pub struct UseCmd {
    state: Arc<State>,
}

impl UseCmd {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for UseCmd {
    fn name(&self) -> &str {
        "use"
    }

    fn signature(&self) -> Signature {
        Signature::build("use")
    }

    fn usage(&self) -> &str {
        "Modify the default execution environment of commands"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        use_cmd(args, self.state.clone())
    }
}

fn use_cmd(args: CommandArgs, state: Arc<State>) -> Result<OutputStream, ShellError> {
    let _args = args.evaluate_once()?;

    let active = state.active_cluster();
    let mut using_now = TaggedDictBuilder::new(Tag::default());
    using_now.insert_value("username", active.username());
    using_now.insert_value("cluster", state.active());
    using_now.insert_value(
        "bucket",
        active
            .active_bucket()
            .map(|s| s.clone())
            .unwrap_or(String::from("<not set>")),
    );
    using_now.insert_value(
        "scope",
        active
            .active_scope()
            .map(|s| s.clone())
            .unwrap_or(String::from("")),
    );
    using_now.insert_value(
        "collection",
        active
            .active_collection()
            .map(|s| s.clone())
            .unwrap_or(String::from("")),
    );
    let clusters = vec![using_now.into_value()];

    Ok(clusters.into())
}
