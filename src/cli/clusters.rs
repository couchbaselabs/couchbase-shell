use crate::cli::error::client_error_to_shell_error;
use crate::cli::util::{convert_json_value_to_nu_value, find_org_id, find_project_id, NuValueMap};
use crate::state::State;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

#[derive(Clone)]
pub struct Clusters {
    state: Arc<Mutex<State>>,
}

impl Clusters {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for Clusters {
    fn name(&self) -> &str {
        "clusters"
    }

    fn signature(&self) -> Signature {
        Signature::build("clusters")
            .named(
                "capella",
                SyntaxShape::String,
                "the Capella organization to use",
                None,
            )
            .named(
                "project",
                SyntaxShape::String,
                "the Capella project to use",
                None,
            )
            .category(Category::Custom("couchbase".to_string()))
    }

    fn usage(&self) -> &str {
        "Lists all clusters on the active Capella organisation"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        clusters(self.state.clone(), engine_state, stack, call, input)
    }
}

fn clusters(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();
    let capella = call.get_flag(engine_state, stack, "capella")?;

    let guard = state.lock().unwrap();
    let control = guard.named_or_active_org(capella)?;

    let project =
        guard.named_or_active_project(call.get_flag(engine_state, stack, "project")?)?;

    let client = control.client();
    let deadline = Instant::now().add(control.timeout());

    let org_id = find_org_id(ctrl_c.clone(), &client, deadline, span)?;
    let project_id = find_project_id(
        ctrl_c.clone(),
        project,
        &client,
        deadline,
        span,
        org_id.clone(),
    )?;

    let clusters = client
        .list_clusters(org_id, project_id, deadline, ctrl_c)
        .map_err(|e| client_error_to_shell_error(e, span))?;

    let mut results = vec![];
    for cluster in clusters.items() {
        let mut collected = NuValueMap::default();
        collected.add_string("name", cluster.name(), span);
        collected.add_string("id", cluster.id(), span);
        collected.add_string("state", cluster.state(), span);
        collected.add(
            "cloud provider",
            convert_json_value_to_nu_value(
                &serde_json::to_value(cluster.cloud_provider()).unwrap(),
                span,
            )
            .unwrap(),
        );
        results.push(collected.into_value(span))
    }

    Ok(Value::List {
        vals: results,
        internal_span: span,
    }
    .into_pipeline_data())
}
