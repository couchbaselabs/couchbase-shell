use crate::state::State;
use async_trait::async_trait;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, TaggedDictBuilder};
use nu_source::Tag;
use nu_stream::OutputStream;
use std::sync::{Arc, Mutex};

pub struct UseProject {
    state: Arc<Mutex<State>>,
}

impl UseProject {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for UseProject {
    fn name(&self) -> &str {
        "use project"
    }

    fn signature(&self) -> Signature {
        Signature::build("use project").required(
            "identifier",
            SyntaxShape::String,
            "the name of the project",
        )
    }

    fn usage(&self) -> &str {
        "Sets the active project based on its name"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let guard = self.state.lock().unwrap();
        let active = guard.active_capella_org()?;

        active.set_active_project(args.req(0)?);

        let mut using_now = TaggedDictBuilder::new(Tag::default());
        using_now.insert_value(
            "project",
            active.active_project().unwrap_or_else(|| String::from("")),
        );
        let clusters = vec![using_now.into_value()];
        Ok(clusters.into())
    }
}
