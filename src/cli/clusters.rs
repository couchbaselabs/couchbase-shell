use crate::cli::cloud_json::JSONCloudClustersSummariesV3;
use crate::client::CapellaRequest;
use crate::state::State;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

use crate::cli::util::{
    generic_unspanned_error, map_serde_deserialize_error_to_shell_error, NuValueMap,
};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};

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
            .category(Category::Custom("couchbase".into()))
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

    let control = if let Some(c) = capella {
        guard.capella_org_for_cluster(c)
    } else {
        guard.active_capella_org()
    }?;
    let client = control.client();

    let response = client.capella_request(
        CapellaRequest::GetClustersV3 {},
        Instant::now().add(control.timeout()),
        ctrl_c,
    )?;
    if response.status() != 200 {
        return Err(generic_unspanned_error(
            "Failed to get clusters",
            format!("Failed to get clusters {}", response.content()),
        ));
    };

    let content: JSONCloudClustersSummariesV3 = serde_json::from_str(response.content())
        .map_err(map_serde_deserialize_error_to_shell_error)?;

    let mut results = vec![];
    for cluster in content.items() {
        let mut collected = NuValueMap::default();
        collected.add_string("name", cluster.name(), span);
        collected.add_string("id", cluster.id(), span);
        collected.add_string("cloud_id", cluster.cloud_id(), span);
        collected.add_string("project_id", cluster.project_id(), span);
        collected.add_string("environment", cluster.environment(), span);
        collected.add_string("tenant_id", content.tenant_id(), span);
        results.push(collected.into_value(span))
    }

    Ok(Value::List {
        vals: results,
        span: call.head,
    }
    .into_pipeline_data())
}
