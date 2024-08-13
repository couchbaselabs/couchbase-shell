use crate::cli::util::NuValueMap;
use crate::state::State;
use std::sync::{Arc, Mutex};

use log::debug;
use std::ops::Add;
use tokio::time::Instant;

use crate::cli::error::client_error_to_shell_error;
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
    let orgs = guard.capella_orgs();
    let mut results = vec![];
    for (identifier, org) in orgs.iter() {
        let client = org.client();
        let mut collected = NuValueMap::default();
        collected.add_string("identifier", identifier, span);

        let deadline = Instant::now().add(org.timeout());
        let orgs = client
            .list_organizations(deadline, ctrl_c.clone())
            .map_err(|e| client_error_to_shell_error(e, span))?;

        for org in orgs.items() {
            collected.add_string("name", org.name(), span);
            collected.add_string("id", org.id(), span);
        }

        results.push(collected.into_value(span))
    }

    Ok(Value::List {
        vals: results,
        internal_span: span,
    }
    .into_pipeline_data())
}
