use crate::state::State;
use async_trait::async_trait;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, UntaggedValue};
use nu_source::Tag;
use nu_stream::OutputStream;
use std::sync::{Arc, Mutex};

pub struct TutorialPrev {
    state: Arc<Mutex<State>>,
}

impl TutorialPrev {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
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

    fn run(&self, _args: CommandArgs) -> Result<OutputStream, ShellError> {
        run_tutorial_prev(self.state.clone())
    }
}

fn run_tutorial_prev(state: Arc<Mutex<State>>) -> Result<OutputStream, ShellError> {
    let guard = state.lock().unwrap();
    let tutorial = guard.tutorial();
    Ok(OutputStream::one(
        UntaggedValue::string(tutorial.prev_tutorial_step()).into_value(Tag::unknown()),
    ))
}
