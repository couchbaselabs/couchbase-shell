use crate::state::State;
use nu_engine::CallExt;
use std::sync::{Arc, Mutex};

use crate::cli::util::NuValueMap;
use nu_engine::command_prelude::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct TutorialPage {
    state: Arc<Mutex<State>>,
}

impl TutorialPage {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for TutorialPage {
    fn name(&self) -> &str {
        "tutorial page"
    }

    fn signature(&self) -> Signature {
        Signature::build("tutorial page")
            .optional("name", SyntaxShape::String, "the name of the page to go to")
            .category(Category::Custom("couchbase".to_string()))
    }

    fn usage(&self) -> &str {
        "Step to a specific page in the Couchbase Shell tutorial"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        run_tutorial_page(self.state.clone(), engine_state, stack, call)
    }
}

fn run_tutorial_page(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<PipelineData, ShellError> {
    let name: Option<String> = call.opt(engine_state, stack, 0)?;

    let guard = state.lock().unwrap();
    let tutorial = guard.tutorial();
    if let Some(n) = name {
        Ok(Value::String {
            val: tutorial.goto_step(n)?,
            internal_span: call.head,
        }
        .into_pipeline_data())
    } else {
        let mut results: Vec<Value> = vec![];
        let (current_step, steps) = tutorial.step_names();
        for s in steps {
            let mut collected = NuValueMap::default();
            let mut step_name = s.clone();
            if s == current_step {
                step_name += " (active)";
            }
            collected.add_string("page_name", step_name, call.head);
            results.push(collected.into_value(call.head));
        }

        Ok(Value::List {
            vals: results,
            internal_span: call.head,
        }
        .into_pipeline_data())
    }
}
