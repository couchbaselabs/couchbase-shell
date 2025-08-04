use crate::state::State;
use std::sync::{Arc, Mutex};

use nu_engine::command_prelude::Call;
use nu_engine::CallExt;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::Value::Nothing;
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape};

#[derive(Clone)]
pub struct UseCapellaOrganization {
    state: Arc<Mutex<State>>,
}

impl UseCapellaOrganization {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for UseCapellaOrganization {
    fn name(&self) -> &str {
        "cb-env capella-organization"
    }

    fn signature(&self) -> Signature {
        Signature::build("cb-env capella-organization")
            .required(
                "identifier",
                SyntaxShape::String,
                "the identifier of the capella organization",
            )
            .category(Category::Custom("couchbase".to_string()))
    }

    fn description(&self) -> &str {
        "Sets the active capella organization based on its identifier"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let guard = self.state.lock().unwrap();
        guard.set_active_capella_org(call.req(engine_state, stack, 0)?)?;

        Ok(PipelineData::Value(
            Nothing {
                internal_span: call.head,
            },
            None,
        ))
    }
}
