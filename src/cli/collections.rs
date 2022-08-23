use crate::cli::util::{cluster_identifiers_from, get_active_cluster, NuValueMap};
use crate::client::ManagementRequest;
use crate::state::State;
use log::debug;
use serde_derive::Deserialize;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::Instant;

use crate::cli::error::{deserialize_error, no_active_bucket_error, unexpected_status_code_error};
use crate::RemoteCluster;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct Collections {
    state: Arc<Mutex<State>>,
}

impl Collections {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for Collections {
    fn name(&self) -> &str {
        "collections"
    }

    fn signature(&self) -> Signature {
        Signature::build("collections")
            .named(
                "bucket",
                SyntaxShape::String,
                "the name of the bucket",
                None,
            )
            .named("scope", SyntaxShape::String, "the name of the scope", None)
            .named(
                "clusters",
                SyntaxShape::String,
                "the clusters to query against",
                None,
            )
            .category(Category::Custom("couchbase".to_string()))
    }

    fn usage(&self) -> &str {
        "Fetches collections through the HTTP API"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        collections_get(self.state.clone(), engine_state, stack, call, input)
    }
}

fn collections_get(
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

    let scope: Option<String> = call.get_flag(engine_state, stack, "scope")?;

    let mut results: Vec<Value> = vec![];
    for identifier in cluster_identifiers {
        let active_cluster = get_active_cluster(identifier.clone(), &guard, span)?;

        let bucket = get_bucket_or_active(active_cluster, engine_state, stack, call)?;

        debug!(
            "Running collections get for bucket {:?}, scope {:?}",
            &bucket, &scope
        );

        let response = active_cluster.cluster().http_client().management_request(
            ManagementRequest::GetCollections { bucket },
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

        for scope_res in manifest.scopes {
            if let Some(scope_name) = &scope {
                if scope_name != &scope_res.name {
                    continue;
                }
            }
            let collections = scope_res.collections;
            if collections.is_empty() {
                let mut collected = NuValueMap::default();
                collected.add_string("scope", scope_res.name.clone(), span);
                collected.add_string("collection", "", span);
                collected.add("max_expiry", Value::Duration { val: 0, span });
                collected.add_string("cluster", identifier.clone(), span);
                results.push(collected.into_value(span));
                continue;
            }

            for collection in collections {
                let mut collected = NuValueMap::default();
                collected.add_string("scope", scope_res.name.clone(), span);
                collected.add_string("collection", collection.name, span);
                collected.add(
                    "max_expiry",
                    Value::Duration {
                        val: Duration::from_secs(collection.max_expiry).as_nanos() as i64,
                        span,
                    },
                );
                collected.add_string("cluster", identifier.clone(), span);
                results.push(collected.into_value(span));
            }
        }
    }

    Ok(Value::List {
        vals: results,
        span,
    }
    .into_pipeline_data())
}

pub fn get_bucket_or_active(
    active_cluster: &RemoteCluster,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<String, ShellError> {
    match call.get_flag(engine_state, stack, "bucket")? {
        Some(v) => Ok(v),
        None => match active_cluster.active_bucket() {
            Some(s) => Ok(s),
            None => Err(no_active_bucket_error(call.span())),
        },
    }
}

#[derive(Debug, Deserialize)]
pub struct ManifestCollection {
    pub uid: String,
    pub name: String,
    #[serde(rename = "maxTTL")]
    pub max_expiry: u64,
}

#[derive(Debug, Deserialize)]
pub struct ManifestScope {
    pub uid: String,
    pub name: String,
    pub collections: Vec<ManifestCollection>,
}

#[derive(Debug, Deserialize)]
pub struct Manifest {
    pub uid: String,
    pub scopes: Vec<ManifestScope>,
}
