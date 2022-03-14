use crate::cli::collections::Manifest;
use crate::cli::util::{
    cluster_identifiers_from, cluster_not_found_error, generic_labeled_error,
    map_serde_deserialize_error_to_shell_error, validate_is_not_cloud, NuValueMap,
};
use crate::client::ManagementRequest;
use crate::state::State;
use log::debug;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct Scopes {
    state: Arc<Mutex<State>>,
}

impl Scopes {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for Scopes {
    fn name(&self) -> &str {
        "scopes"
    }

    fn signature(&self) -> Signature {
        Signature::build("scopes")
            .named(
                "bucket",
                SyntaxShape::String,
                "the name of the bucket",
                None,
            )
            .named(
                "clusters",
                SyntaxShape::String,
                "the clusters to query against",
                None,
            )
            .category(Category::Custom("couchbase".into()))
    }

    fn usage(&self) -> &str {
        "Fetches scopes through the HTTP API"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        run(self.state.clone(), engine_state, stack, call, input)
    }
}

fn run(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();

    let cluster_identifiers = cluster_identifiers_from(&engine_state, stack, &state, &call, true)?;

    let guard = state.lock().unwrap();

    let mut results: Vec<Value> = vec![];
    for identifier in cluster_identifiers {
        let active_cluster = match guard.clusters().get(&identifier) {
            Some(c) => c,
            None => {
                return Err(cluster_not_found_error(identifier));
            }
        };
        validate_is_not_cloud(
            active_cluster,
            "scopes get cannot be run against Capella clusters",
        )?;

        let bucket = match call.get_flag(engine_state, stack, "bucket")? {
            Some(v) => v,
            None => match active_cluster.active_bucket() {
                Some(s) => s,
                None => {
                    return Err(ShellError::MissingParameter(
                        "Could not auto-select a bucket - please use --bucket instead".to_string(),
                        span,
                    ));
                }
            },
        };

        debug!("Running scopes get for bucket {:?}", &bucket);

        let response = active_cluster.cluster().http_client().management_request(
            ManagementRequest::GetScopes { bucket },
            Instant::now().add(active_cluster.timeouts().management_timeout()),
            ctrl_c.clone(),
        )?;

        let manifest: Manifest = match response.status() {
            200 => serde_json::from_str(response.content())
                .map_err(map_serde_deserialize_error_to_shell_error)?,
            _ => {
                return Err(generic_labeled_error(
                    "Failed to get scopes",
                    format!("Failed to get scopes {}", response.content()),
                ));
            }
        };

        for scope in manifest.scopes {
            let mut collected = NuValueMap::default();
            collected.add_string("scope", scope.name, span);
            collected.add_string("cluster", identifier.clone(), span);
            results.push(collected.into_value(span));
        }
    }

    Ok(Value::List {
        vals: results,
        span: call.head,
    }
    .into_pipeline_data())
}
