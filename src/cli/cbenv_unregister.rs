use crate::cli::cbenv_register::update_config_file;
use crate::state::State;
use std::sync::{Arc, Mutex};

use crate::cli::error::{cluster_not_found_error, generic_error};
use nu_engine::command_prelude::Call;
use nu_engine::CallExt;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::Value::Nothing;
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape};

#[derive(Clone)]
pub struct CbEnvUnregister {
    state: Arc<Mutex<State>>,
}

impl CbEnvUnregister {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for CbEnvUnregister {
    fn name(&self) -> &str {
        "cb-env unregister"
    }

    fn signature(&self) -> Signature {
        Signature::build("cb-env unregister")
            .required(
                "identifier",
                SyntaxShape::String,
                "the identifier to use for this cluster",
            )
            .switch(
                "save",
                "whether or not to remove the cluster from the .cbsh config file, defaults to false",
                None,
            )
            .category(Category::Custom("couchbase".to_string()))
    }

    fn usage(&self) -> &str {
        "Unregisters a cluster for use with the shell"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        clusters_unregister(self.state.clone(), engine_state, stack, call, input)
    }
}

fn clusters_unregister(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let identifier: String = call.req(engine_state, stack, 0)?;
    let save = call.get_flag(engine_state, stack, "save")?.unwrap_or(false);

    let mut guard = state.lock().unwrap();
    if guard.active() == identifier {
        return Err(generic_error(
            "Cannot unregister the active cluster",
            "Change the active cluster using cb-env cluster first".to_string(),
            call.head,
        ));
    }

    if guard.remove_cluster(identifier.clone()).is_none() {
        return Err(cluster_not_found_error(identifier, call.span()));
    };

    if save {
        update_config_file(&mut guard, call.head)?;
    };

    Ok(PipelineData::Value(
        Nothing {
            internal_span: call.head,
        },
        None,
    ))
}
