use crate::state::State;
use async_trait::async_trait;
use nu_cli::OutputStream;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue};
use nu_source::Tag;
use std::sync::Arc;

pub struct TutorialPrev {
    state: Arc<State>,
}

impl TutorialPrev {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for TutorialPrev {
    fn name(&self) -> &str {
        "tutorial prev"
    }

    fn signature(&self) -> Signature {
        Signature::build("tutorial prev")
    }

    fn usage(&self) -> &str {
        "Step to the previous page in the Couchbase Shell tutorial"
    }

    async fn run(&self, _args: CommandArgs) -> Result<OutputStream, ShellError> {
        run_tutorial_prev(self.state.clone()).await
    }
}

async fn run_tutorial_prev(state: Arc<State>) -> Result<OutputStream, ShellError> {
    let tutorial = state.tutorial();
    Ok(OutputStream::one(ReturnSuccess::value(
        UntaggedValue::string(tutorial.prev_tutorial_step()).into_value(Tag::unknown()),
    )))
}
