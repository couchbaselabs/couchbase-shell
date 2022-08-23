use crate::cli::collections::{get_bucket_or_active, Manifest};
use crate::cli::error::{deserialize_error, unexpected_status_code_error};
use crate::cli::util::{cluster_identifiers_from, get_active_cluster, NuValueMap};
use crate::client::ManagementRequest;
use crate::state::State;
use log::debug;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

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
            .category(Category::Custom("couchbase".to_string()))
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

    let cluster_identifiers = cluster_identifiers_from(engine_state, stack, &state, call, true)?;

    let guard = state.lock().unwrap();

    let mut results: Vec<Value> = vec![];
    for identifier in cluster_identifiers {
        let active_cluster = get_active_cluster(identifier.clone(), &guard, span)?;

        let bucket = get_bucket_or_active(active_cluster, engine_state, stack, call)?;

        debug!("Running scopes get for bucket {:?}", &bucket);

        let response = active_cluster.cluster().http_client().management_request(
            ManagementRequest::GetScopes { bucket },
            Instant::now().add(active_cluster.timeouts().management_timeout()),
            ctrl_c.clone(),
        )?;

        let manifest: Manifest = match response.status() {
            200 => serde_json::from_str(response.content())
                .map_err(|e| deserialize_error(e.to_string(), span))?,
            _ => {
                return Err(unexpected_status_code_error(
                    response.status(),
                    response.content(),
                    span,
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
        span,
    }
    .into_pipeline_data())
}
