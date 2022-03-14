use crate::cli::util::NuValueMap;
use crate::state::State;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct UseScope {
    state: Arc<Mutex<State>>,
}

impl UseScope {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for UseScope {
    fn name(&self) -> &str {
        "cb-env scope"
    }

    fn signature(&self) -> Signature {
        Signature::build("cb-env scope")
            .required("identifier", SyntaxShape::String, "the name of the scope")
            .category(Category::Custom("couchbase".into()))
    }

    fn usage(&self) -> &str {
        "Sets the active scope based on its name"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let guard = self.state.lock().unwrap();
        let active = match guard.active_cluster() {
            Some(c) => c,
            None => {
                return Err(ShellError::LabeledError(
                    "An active cluster must be set".into(),
                    "No active cluser was found".into(),
                ));
            }
        };

        if active.active_bucket().is_none() {
            return Err(ShellError::LabeledError(
                "You must select a bucket before a collection".into(),
                "You must select a bucket before a collection".into(),
            ));
        }

        active.set_active_scope(call.req(engine_state, stack, 0)?);

        let mut result = NuValueMap::default();
        result.add_string(
            "scope",
            active
                .active_scope()
                .unwrap_or_else(|| String::from("<not set>")),
            call.head,
        );

        Ok(result.into_pipeline_data(call.head))
    }
}
