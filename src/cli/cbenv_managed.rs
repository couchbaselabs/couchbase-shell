use crate::cli::util::NuValueMap;
use crate::state::State;
use std::sync::{Arc, Mutex};

use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, IntoPipelineData, PipelineData, ShellError, Signature, Value};

#[derive(Clone)]
pub struct CBEnvManaged {
    state: Arc<Mutex<State>>,
}

impl CBEnvManaged {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for CBEnvManaged {
    fn name(&self) -> &str {
        "cb-env managed"
    }

    fn signature(&self) -> Signature {
        Signature::build("cb-env managed").category(Category::Custom("couchbase".to_string()))
    }

    fn usage(&self) -> &str {
        "Lists all clusters currently managed by couchbase shell"
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
    _engine_state: &EngineState,
    _stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;

    let guard = state.lock().unwrap();
    let active = guard.active();
    let clusters = guard
        .clusters()
        .iter()
        .map(|(k, v)| {
            let mut collected = NuValueMap::default();
            collected.add_bool("active", k == &active, span);
            collected.add_bool("tls", v.tls_config().enabled(), span);
            collected.add_string("identifier", k.clone(), span);
            collected.add_string("username", String::from(v.username()), span);
            collected.add_string(
                "capella_organization",
                v.capella_org().unwrap_or_else(|| "".to_string()),
                span,
            );
            collected.into_value(span)
        })
        .collect::<Vec<_>>();

    Ok(Value::List {
        vals: clusters,
        span,
    }
    .into_pipeline_data())
}
