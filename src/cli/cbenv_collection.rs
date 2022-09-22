use crate::cli::error::{no_active_bucket_error, no_active_cluster_error};
use crate::cli::util::NuValueMap;
use crate::state::State;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct UseCollection {
    state: Arc<Mutex<State>>,
}

impl UseCollection {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for UseCollection {
    fn name(&self) -> &str {
        "cb-env collection"
    }

    fn signature(&self) -> Signature {
        Signature::build("cb-env collection")
            .required(
                "identifier",
                SyntaxShape::String,
                "the name of the collection",
            )
            .category(Category::Custom("couchbase".to_string()))
    }

    fn usage(&self) -> &str {
        "Sets the active collection based on its name"
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

        active.set_active_collection(call.req(engine_state, stack, 0)?);

        let mut result = NuValueMap::default();
        result.add_string(
            "collection",
            active
                .active_collection()
                .unwrap_or_else(|| String::from("")),
            span,
        );

        Ok(result.into_pipeline_data(span))
    }
}
