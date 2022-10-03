use crate::state::State;
use std::sync::{Arc, Mutex};

use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, IntoPipelineData, PipelineData, ShellError, Signature, Value};

#[derive(Clone)]
pub struct Tutorial {
    state: Arc<Mutex<State>>,
}

impl Tutorial {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for Tutorial {
    fn name(&self) -> &str {
        "tutorial"
    }

    fn signature(&self) -> Signature {
        Signature::build("tutorial").category(Category::Custom("couchbase".to_string()))
    }

    fn usage(&self) -> &str {
        "Run the Couchbase Shell tutorial"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        run_tutorial(self.state.clone(), engine_state, stack, call, input)
    }
}

fn run_tutorial(
    state: Arc<Mutex<State>>,
    _engine_state: &EngineState,
    _stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let guard = state.lock().unwrap();
    let tutorial = guard.tutorial();

    Ok(Value::String {
        val: tutorial.current_step(),
        span: call.head,
    }
    .into_pipeline_data())
}
