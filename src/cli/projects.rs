use crate::cli::util::find_org_id;
use crate::cli::util::NuValueMap;
use crate::state::State;
use log::debug;
use nu_engine::CallExt;
use std::sync::{Arc, Mutex};

use crate::cli::error::client_error_to_shell_error;
use nu_engine::command_prelude::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, IntoPipelineData, PipelineData, ShellError, Signature, Value};

#[derive(Clone)]
pub struct Projects {
    state: Arc<Mutex<State>>,
}

impl Projects {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for Projects {
    fn name(&self) -> &str {
        "projects"
    }

    fn signature(&self) -> Signature {
        Signature::build("projects")
            .category(Category::Custom("couchbase".to_string()))
            .switch("audit", "display audit data for the projects", None)
    }

    fn usage(&self) -> &str {
        "Lists all Capella projects"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        projects(self.state.clone(), engine_state, stack, call, input)
    }
}

fn projects(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let signals = engine_state.signals().clone();
    let audit = call.has_flag(engine_state, stack, "audit")?;

    debug!("Running projects");

    let guard = &mut state.lock().unwrap();
    let control = guard.active_capella_org()?;
    let client = control.client();

    let org_id = find_org_id(signals.clone(), &client, span)?;

    let projects = client
        .list_projects(org_id, signals)
        .map_err(|e| client_error_to_shell_error(e, span))?;

    let mut results = vec![];
    for project in projects.items() {
        let mut collected = NuValueMap::default();
        collected.add_string("name", project.name(), span);
        collected.add_string("id", project.id(), span);

        if audit {
            collected.add_string("created by", project.created_by(), span);
            collected.add_string("created at", project.created_at(), span);
            collected.add_string("modified by", project.modified_by(), span);
            collected.add_string("modified at", project.modified_at(), span);
        }

        results.push(collected.into_value(span))
    }

    Ok(Value::List {
        vals: results,
        internal_span: span,
    }
    .into_pipeline_data())
}
