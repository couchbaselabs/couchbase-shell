use crate::state::State;
use std::sync::{Arc, Mutex};

use crate::cli::util::NuValueMap;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape};

#[derive(Clone)]
pub struct UseCloud {
    state: Arc<Mutex<State>>,
}

impl UseCloud {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for UseCloud {
    fn name(&self) -> &str {
        "cb-env cloud"
    }

    fn signature(&self) -> Signature {
        Signature::build("cb-env cloud")
            .required(
                "identifier",
                SyntaxShape::String,
                "the identifier of the cloud",
            )
            .category(Category::Custom("couchbase".into()))
    }

    fn usage(&self) -> &str {
        "Sets the active cloud on the active capella organisation, based on its identifier"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let cloud_name: String = call.req(engine_state, stack, 0)?;
        let guard = self.state.lock().unwrap();
        let org = guard.active_capella_org()?;
        org.set_active_cloud(cloud_name.clone());

        let mut result = NuValueMap::default();
        result.add_string("cloud", cloud_name, call.head);

        Ok(result.into_pipeline_data(call.head))
    }
}
