use crate::state::State;

use nu_engine::command_prelude::Call;
use nu_engine::CallExt;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape, Value};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct UseProject {
    state: Arc<Mutex<State>>,
}

impl UseProject {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for UseProject {
    fn name(&self) -> &str {
        "cb-env project"
    }

    fn signature(&self) -> Signature {
        Signature::build("cb-env project")
            .required("identifier", SyntaxShape::String, "the name of the project")
            .category(Category::Custom("couchbase".to_string()))
    }

    fn description(&self) -> &str {
        "Sets the active project based on its name"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let guard = self.state.lock().unwrap();
        let project: String = call.req(engine_state, stack, 0)?;
        guard.set_active_project(project);

        Ok(PipelineData::Value(Value::nothing(call.head), None))
    }
}
