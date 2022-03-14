use super::util::convert_json_value_to_nu_value;
use crate::cli::util::{
    cluster_identifiers_from, cluster_not_found_error, map_serde_deserialize_error_to_shell_error,
    validate_is_not_cloud,
};
use crate::client::ManagementRequest;
use crate::state::State;
use serde_json::{json, Map, Value};
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

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
            .category(Category::Custom("couchbase".into()))
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

    let cluster_identifiers = cluster_identifiers_from(&engine_state, stack, &state, &call, true)?;

    let mut entries = vec![];
    for identifier in cluster_identifiers {
        let guard = state.lock().unwrap();
        let cluster = match guard.clusters().get(&identifier) {
            Some(c) => c,
            None => {
                return Err(cluster_not_found_error(identifier));
            }
        };
        validate_is_not_cloud(cluster, "whoami cannot be run against cloud clusters")?;

        let response = cluster.cluster().http_client().management_request(
            ManagementRequest::Whoami,
            Instant::now().add(cluster.timeouts().management_timeout()),
            ctrl_c.clone(),
        )?;
        let mut content: Map<String, Value> = serde_json::from_str(response.content())
            .map_err(map_serde_deserialize_error_to_shell_error)?;
        content.insert("cluster".into(), json!(identifier.clone()));
        let converted = convert_json_value_to_nu_value(&Value::Object(content), span)?;
        entries.push(converted);
    }

    Ok(NuValue::List {
        vals: entries,
        span,
    }
    .into_pipeline_data())
}
