use crate::cli::util::NuValueMap;
use crate::state::State;

use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape};
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
            .category(Category::Custom("couchbase".into()))
    }

    fn usage(&self) -> &str {
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
        let active = guard.active_capella_org()?;

        active.set_active_project(call.req(engine_state, stack, 0)?);

        let mut result = NuValueMap::default();
        result.add_string(
            "project",
            active.active_project().unwrap_or_else(|| String::from("")),
            call.head,
        );

        Ok(result.into_pipeline_data(call.head))
    }
}
