use crate::state::State;
use log::debug;
use std::sync::{Arc, Mutex};

use crate::cli::error::client_error_to_shell_error;
use crate::cli::util::{convert_json_value_to_nu_value, find_org_id, find_project_id, NuValueMap};
use nu_engine::command_prelude::Call;
use nu_engine::CallExt;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape};

#[derive(Clone)]
pub struct ClustersGet {
    state: Arc<Mutex<State>>,
}

impl ClustersGet {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for ClustersGet {
    fn name(&self) -> &str {
        "clusters get"
    }

    fn signature(&self) -> Signature {
        Signature::build("clusters get")
            .required("name", SyntaxShape::String, "the name of the cluster")
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

    fn description(&self) -> &str {
        "Gets a cluster from the active Capella Project"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        clusters_get(self.state.clone(), engine_state, stack, call, input)
    }
}

fn clusters_get(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let signals = engine_state.signals().clone();

    let name: String = call.req(engine_state, stack, 0)?;
    let capella = call.get_flag(engine_state, stack, "capella")?;

    debug!("Running clusters get for {}", &name);

    let guard = state.lock().unwrap();
    let control = guard.named_or_active_org(capella)?;

    let project =
        guard.named_or_active_project(call.get_flag(engine_state, stack, "project")?)?;

    let client = control.client();

    let org_id = find_org_id(signals.clone(), &client, span)?;
    let project_id = find_project_id(signals.clone(), project, &client, span, org_id.clone())?;

    let cluster = client
        .get_cluster(name, org_id.clone(), project_id.clone(), signals.clone())
        .map_err(|e| client_error_to_shell_error(e, span))?;

    let mut collected = NuValueMap::default();
    collected.add_string("name", cluster.name(), span);
    collected.add_string("id", cluster.id(), span);
    collected.add_string("description", cluster.description(), span);
    collected.add_string("state", cluster.state(), span);
    collected.add_string("connection string", cluster.connection_string(), span);
    collected.add_string("configuration type", cluster.configuration_type(), span);
    collected.add(
        "server",
        convert_json_value_to_nu_value(
            &serde_json::to_value(cluster.couchbase_server()).unwrap(),
            span,
        )
        .unwrap(),
    );
    collected.add(
        "cloud provider",
        convert_json_value_to_nu_value(
            &serde_json::to_value(cluster.cloud_provider()).unwrap(),
            span,
        )
        .unwrap(),
    );
    collected.add(
        "service groups",
        convert_json_value_to_nu_value(
            &serde_json::to_value(cluster.service_groups()).unwrap(),
            span,
        )
        .unwrap(),
    );
    collected.add(
        "availability",
        convert_json_value_to_nu_value(
            &serde_json::to_value(cluster.availability()).unwrap(),
            span,
        )
        .unwrap(),
    );
    collected.add(
        "support",
        convert_json_value_to_nu_value(&serde_json::to_value(cluster.support()).unwrap(), span)
            .unwrap(),
    );
    if let Some(audit) = cluster.audit_data() {
        collected.add(
            "audit data",
            convert_json_value_to_nu_value(&serde_json::to_value(audit).unwrap(), span).unwrap(),
        );
    }
    if let Some(app_service_id) = cluster.app_service_id() {
        collected.add_string("app service id", app_service_id, span);
    }
    if let Some(cmek_id) = cluster.cmek_id() {
        collected.add_string("cmek id", cmek_id, span);
    }

    Ok(collected.into_pipeline_data(span))
}
