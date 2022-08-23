use crate::cli::clusters_register::update_config_file;
use crate::state::State;
use std::sync::{Arc, Mutex};

use crate::cli::error::{cluster_not_found_error, generic_error};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape};

#[derive(Clone)]
pub struct ClustersUnregister {
    state: Arc<Mutex<State>>,
}

impl ClustersUnregister {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for ClustersUnregister {
    fn name(&self) -> &str {
        "clusters unregister"
    }

    fn signature(&self) -> Signature {
        Signature::build("clusters unregister")
            .required(
                "identifier",
                SyntaxShape::String,
                "the identifier to use for this cluster",
            )
            .switch(
                "save",
                "whether or not to add the cluster to the .cbsh config file, defaults to false",
                None,
            )
            .category(Category::Custom("couchbase".to_string()))
    }

    fn usage(&self) -> &str {
        "Registers a cluster for use with the shell"
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
    if guard.active() == identifier.clone() {
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

    return Ok(PipelineData::new(call.head));
}
