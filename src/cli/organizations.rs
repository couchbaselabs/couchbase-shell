use crate::cli::cloud_json::JSONCloudsOrganizationsResponse;
use crate::cli::util::NuValueMap;
use crate::client::CapellaRequest;
use crate::state::State;
use std::sync::{Arc, Mutex};

use log::debug;
use std::ops::Add;
use tokio::time::Instant;

use crate::cli::error::{
    client_error_to_shell_error, deserialize_error, unexpected_status_code_error,
};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, IntoPipelineData, PipelineData, ShellError, Signature, Value};

#[derive(Clone)]
pub struct Organizations {
    state: Arc<Mutex<State>>,
}

impl Organizations {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for Organizations {
    fn name(&self) -> &str {
        "organizations"
    }

    fn signature(&self) -> Signature {
        Signature::build("organizations").category(Category::Custom("couchbase".to_string()))
    }

    fn usage(&self) -> &str {
        "Lists all organizations the user has access too"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        organizations(self.state.clone(), engine_state, stack, call, input)
    }
}

fn organizations(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    _stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();

    debug!("Running organizations");

    let guard = state.lock().unwrap();
    let control = guard.active_capella_org()?;
    let client = control.client();
    let response = client
        .capella_request(
            CapellaRequest::GetOrganizations {},
            Instant::now().add(control.timeout()),
            ctrl_c,
        )
        .map_err(|e| client_error_to_shell_error(e, span))?;
    if response.status() != 200 {
        return Err(unexpected_status_code_error(
            response.status(),
            response.content(),
            span,
        ));
    }

    let content: JSONCloudsOrganizationsResponse = serde_json::from_str(response.content())
        .map_err(|e| deserialize_error(e.to_string(), span))?;

    let mut results = vec![];
    for project in content.items() {
        let mut collected = NuValueMap::default();
        collected.add_string("name", project.name(), span);
        collected.add_string("id", project.id(), span);
        results.push(collected.into_value(span))
    }

    Ok(Value::List {
        vals: results,
        span: span,
    }
    .into_pipeline_data())
}
