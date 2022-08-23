use super::util::convert_json_value_to_nu_value;
use crate::cli::util::{cluster_identifiers_from, get_active_cluster};
use crate::client::ManagementRequest;
use crate::state::State;
use serde_json::{json, Map, Value};
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

use crate::cli::error::{deserialize_error, unexpected_status_code_error};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value as NuValue,
};

#[derive(Clone)]
pub struct Whoami {
    state: Arc<Mutex<State>>,
}

impl Whoami {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for Whoami {
    fn name(&self) -> &str {
        "whoami"
    }

    fn signature(&self) -> Signature {
        Signature::build("whoami")
            .named(
                "clusters",
                SyntaxShape::String,
                "the clusters which should be contacted",
                None,
            )
            .category(Category::Custom("couchbase".to_string()))
    }

    fn usage(&self) -> &str {
        "Shows roles and domain for the connected user"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        whoami(self.state.clone(), engine_state, stack, call, input)
    }
}

fn whoami(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();

    let cluster_identifiers = cluster_identifiers_from(engine_state, stack, &state, call, true)?;
    let guard = state.lock().unwrap();

    let mut entries = vec![];
    for identifier in cluster_identifiers {
        let cluster = get_active_cluster(identifier.clone(), &guard, span)?;

        let response = cluster.cluster().http_client().management_request(
            ManagementRequest::Whoami,
            Instant::now().add(cluster.timeouts().management_timeout()),
            ctrl_c.clone(),
        )?;

        match response.status() {
            200 => {}
            _ => {
                return Err(unexpected_status_code_error(
                    response.status(),
                    response.content(),
                    span,
                ));
            }
        }

        let mut content: Map<String, Value> = serde_json::from_str(response.content())
            .map_err(|e| deserialize_error(e.to_string(), span))?;
        content.insert("cluster".to_string(), json!(identifier.clone()));
        let converted = convert_json_value_to_nu_value(&Value::Object(content), span)?;
        entries.push(converted);
    }

    Ok(NuValue::List {
        vals: entries,
        span,
    }
    .into_pipeline_data())
}
