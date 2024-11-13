use crate::cli::error::{no_active_bucket_error, no_active_cluster_error};
use crate::state::State;
use nu_engine::command_prelude::Call;
use nu_engine::CallExt;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::Value::Nothing;
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
            .switch("preserve", "preserve the cb-env collection", Some('p'))
            .category(Category::Custom("couchbase".to_string()))
    }

    fn description(&self) -> &str {
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
        let span = call.head;
        let active = match guard.active_cluster() {
            Some(c) => c,
            None => {
                return Err(no_active_cluster_error(span));
            }
        };

        if active.active_bucket().is_none() {
            return Err(no_active_bucket_error(span));
        }

        active.set_active_scope(Some(call.req(engine_state, stack, 0)?));

        if !call.has_flag(engine_state, stack, "preserve")? {
            active.set_active_collection(None);
        }

        Ok(PipelineData::Value(
            Nothing {
                internal_span: span,
            },
            None,
        ))
    }
}
