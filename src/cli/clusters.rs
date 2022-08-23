use crate::cli::cloud_json::JSONCloudClustersSummariesV3;
use crate::cli::error::{deserialize_error, unexpected_status_code_error};
use crate::cli::util::NuValueMap;
use crate::client::CapellaRequest;
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
        return Err(unexpected_status_code_error(
            response.status(),
            response.content(),
            span,
        ));
    };

    let content: JSONCloudClustersSummariesV3 = serde_json::from_str(response.content())
        .map_err(|e| deserialize_error(e.to_string(), span))?;

    let mut results = vec![];
    for cluster in content.items() {
        let mut collected = NuValueMap::default();
        collected.add_string("name", cluster.name(), span);
        collected.add_string("id", cluster.id(), span);
        collected.add_string("project_id", cluster.project_id(), span);
        collected.add_string("tenant_id", content.tenant_id(), span);
        results.push(collected.into_value(span))
    }

    Ok(Value::List {
        vals: results,
        span,
    }
    .into_pipeline_data())
}
