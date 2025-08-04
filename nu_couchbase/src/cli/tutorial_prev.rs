use crate::state::State;
use std::sync::{Arc, Mutex};

use nu_engine::command_prelude::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, IntoPipelineData, PipelineData, ShellError, Signature, Value};

#[derive(Clone)]
pub struct TutorialPrev {
    state: Arc<Mutex<State>>,
}

impl TutorialPrev {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for TutorialPrev {
    fn name(&self) -> &str {
        "tutorial prev"
    }

    fn signature(&self) -> Signature {
        Signature::build("tutorial prev").category(Category::Custom("couchbase".to_string()))
    }

    fn description(&self) -> &str {
        "Step to the previous page in the Couchbase Shell tutorial"
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        run_tutorial_prev(self.state.clone(), call)
    }
}

fn run_tutorial_prev(state: Arc<Mutex<State>>, call: &Call) -> Result<PipelineData, ShellError> {
    let guard = state.lock().unwrap();
    let tutorial = guard.tutorial();

    Ok(Value::String {
        val: tutorial.prev_tutorial_step(),
        internal_span: call.head,
    }
    .into_pipeline_data())
}
