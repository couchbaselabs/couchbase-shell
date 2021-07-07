use crate::state::State;
use async_trait::async_trait;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, TaggedDictBuilder};
use nu_source::Tag;
use nu_stream::OutputStream;
use std::sync::{Arc, Mutex};

pub struct UseCmd {
    state: Arc<Mutex<State>>,
}

impl UseCmd {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
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

fn use_cmd(_args: CommandArgs, state: Arc<Mutex<State>>) -> Result<OutputStream, ShellError> {
    let guard = state.lock().unwrap();
    let active = guard.active_cluster();
    let mut using_now = TaggedDictBuilder::new(Tag::default());
    using_now.insert_value("username", active.username());
    using_now.insert_value("cluster", guard.active());
    using_now.insert_value(
        "bucket",
        active
            .active_bucket()
            .unwrap_or_else(|| String::from("<not set>")),
    );
    using_now.insert_value(
        "scope",
        active.active_scope().unwrap_or_else(|| String::from("")),
    );
    using_now.insert_value(
        "collection",
        active
            .active_collection()
            .unwrap_or_else(|| String::from("")),
    );
    let clusters = vec![using_now.into_value()];

    Ok(clusters.into())
}
