use crate::state::State;
use async_trait::async_trait;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, TaggedDictBuilder};
use nu_source::Tag;
use nu_stream::OutputStream;
use std::sync::{Arc, Mutex};

pub struct UseCloud {
    state: Arc<Mutex<State>>,
}

impl UseCloud {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for UseCloud {
    fn name(&self) -> &str {
        "use cloud"
    }

    fn signature(&self) -> Signature {
        Signature::build("use cloud").required(
            "identifier",
            SyntaxShape::String,
            "the identifier of the cloud",
        )
    }

    fn usage(&self) -> &str {
        "Sets the active cloud based on its identifier"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let guard = self.state.lock().unwrap();
        guard.set_active_cloud(args.req(0)?)?;

        let mut using_now = TaggedDictBuilder::new(Tag::default());
        using_now.insert_value("cloud", guard.active());
        let cloud = vec![using_now.into_value()];
        Ok(cloud.into())
    }
}
