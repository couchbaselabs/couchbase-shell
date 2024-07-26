use crate::state::State;
use log::debug;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

use crate::cli::error::client_error_to_shell_error;
use crate::cli::util::{find_org_id, find_project_id};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape};

#[derive(Clone)]
pub struct ClustersDrop {
    state: Arc<Mutex<State>>,
}

impl ClustersDrop {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for ClustersDrop {
    fn name(&self) -> &str {
        "clusters drop"
    }

    fn signature(&self) -> Signature {
        Signature::build("clusters drop")
            .required("name", SyntaxShape::String, "the name of the cluster")
            .named(
                "capella",
                SyntaxShape::String,
                "the Capella organization to use",
                None,
            )
            .category(Category::Custom("couchbase".to_string()))
    }

    fn usage(&self) -> &str {
        "Deletes a cluster from the active Capella organization"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        clusters_drop(self.state.clone(), engine_state, stack, call, input)
    }
}

fn clusters_drop(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();

    let name: String = call.req(engine_state, stack, 0)?;
    let capella = call.get_flag(engine_state, stack, "capella")?;

    debug!("Running clusters drop for {}", &name);

    let guard = state.lock().unwrap();
    let control = if let Some(c) = capella {
        guard.get_capella_org(c)
    } else {
        guard.active_capella_org()
    }?;

    let client = control.client();
    let deadline = Instant::now().add(control.timeout());

    let org_id = find_org_id(ctrl_c.clone(), &client, deadline, span)?;
    let project_id = find_project_id(
        ctrl_c.clone(),
        guard.active_project()?,
        &client,
        deadline,
        span,
        org_id.clone(),
    )?;

    let cluster = client
        .get_cluster(
            name,
            org_id.clone(),
            project_id.clone(),
            deadline,
            ctrl_c.clone(),
        )
        .map_err(|e| client_error_to_shell_error(e, span))?;

    client
        .delete_cluster(org_id, project_id, cluster.id(), deadline, ctrl_c)
        .map_err(|e| client_error_to_shell_error(e, span))?;

    Ok(PipelineData::empty())
}
