use crate::cli::cloud_json::JSONCloudsResponse;
use crate::cli::error::{deserialize_error, unexpected_status_code_error};
use crate::cli::util::NuValueMap;
use crate::client::CapellaRequest;
use crate::state::State;
use log::debug;
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
pub struct Clouds {
    state: Arc<Mutex<State>>,
}

impl Clouds {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for Clouds {
    fn name(&self) -> &str {
        "clouds"
    }

    fn signature(&self) -> Signature {
        Signature::build("clouds")
            .named(
                "capella",
                SyntaxShape::String,
                "the Capella organization to use",
                None,
            )
            .category(Category::Custom("couchbase".into()))
    }

    fn usage(&self) -> &str {
        "Shows the current status for all clouds belonging to the active Capella organization"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        clouds(self.state.clone(), engine_state, stack, call, input)
    }
}

fn clouds(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();

    let capella = call.get_flag(engine_state, stack, "capella")?;

    debug!("Running clouds");

    let guard = state.lock().unwrap();
    let control = if let Some(c) = capella {
        guard.capella_org_for_cluster(c)
    } else {
        guard.active_capella_org()
    }?;
    let client = control.client();
    let response = client.capella_request(
        CapellaRequest::GetClouds {},
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

    let content: JSONCloudsResponse = serde_json::from_str(response.content())
        .map_err(|e| deserialize_error(e.to_string(), span))?;

    let mut results = vec![];
    for cloud in content.items().into_iter() {
        let mut collected = NuValueMap::default();
        collected.add_string("identifier", cloud.name(), span);
        collected.add_string("status", cloud.status(), span);
        collected.add_string("region", cloud.region(), span);
        collected.add_string("provider", cloud.provider(), span);
        collected.add_string("cloud_id", cloud.id(), span);
        results.push(collected.into_value(span))
    }

    Ok(Value::List {
        vals: results,
        span,
    }
    .into_pipeline_data())
}
