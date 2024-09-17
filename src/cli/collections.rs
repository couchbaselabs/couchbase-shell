use crate::cli::util::{
    cluster_identifiers_from, find_org_project_cluster_ids, get_active_cluster, NuValueMap,
};
use crate::client::ManagementRequest;
use crate::state::State;
use log::debug;
use serde_derive::Deserialize;
use std::ops::Add;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::Instant;

use crate::cli::error::{
    client_error_to_shell_error, deserialize_error, no_active_bucket_error,
    unexpected_status_code_error,
};
use crate::cli::no_active_scope_error;
use crate::client::cloud::CollectionNamespace;
use crate::client::cloud_json::Collection;
use crate::remote_cluster::RemoteClusterType::Provisioned;
use crate::RemoteCluster;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
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

    let mut results: Vec<Value> = vec![];
    for identifier in cluster_identifiers {
        let active_cluster = get_active_cluster(identifier.clone(), &guard, span)?;

        let bucket = get_bucket_or_active(active_cluster, engine_state, stack, call)?;
        let scope = get_scope_or_active(active_cluster, engine_state, stack, call)?;

        debug!(
            "Running collections get for bucket {:?}, scope {:?}",
            &bucket.clone(),
            &scope
        );

        let collections = if active_cluster.cluster_type() == Provisioned {
            let client = guard
                .named_or_active_org(active_cluster.capella_org())?
                .client();

            let (org_id, project_id, cluster_id) = find_org_project_cluster_ids(
                &client,
                ctrl_c.clone(),
                span,
                identifier.clone(),
                guard.named_or_active_project(active_cluster.project())?,
                active_cluster,
            )?;

            let namespace = CollectionNamespace::new(org_id, project_id, cluster_id, bucket, scope);

            let collections = client
                .list_collections(namespace, ctrl_c.clone())
                .map_err(|e| client_error_to_shell_error(e, span))?;

            collections.items()
        } else {
            get_server_collections(
                active_cluster,
                bucket.clone(),
                scope.clone(),
                ctrl_c.clone(),
                span,
            )?
        };

        for collection in collections {
            let mut collected = NuValueMap::default();
            collected.add_string("collection", collection.name(), span);

            let expiry = match collection.max_expiry() {
                -1 => "".to_string(),
                0 => "inherited".to_string(),
                _ => format!("{:?}", Duration::from_secs(collection.max_expiry() as u64)),
            };

            collected.add(
                "max_expiry",
                Value::String {
                    val: expiry,
                    internal_span: span,
                },
            );
            collected.add_string("cluster", identifier.clone(), span);
            results.push(collected.into_value(span));
        }
    }

    Ok(Value::List {
        vals: results,
        internal_span: span,
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

pub fn get_scope_or_active(
    active_cluster: &RemoteCluster,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<String, ShellError> {
    match call.get_flag(engine_state, stack, "scope")? {
        Some(v) => Ok(v),
        None => match active_cluster.active_scope() {
            Some(s) => Ok(s),
            None => Err(no_active_scope_error(call.span())),
        },
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct ManifestScope {
    pub name: String,
    pub collections: Vec<Collection>,
}

impl ManifestScope {
    pub fn collections(&self) -> Vec<Collection> {
        self.collections.clone()
    }
}

#[derive(Debug, Deserialize)]
pub struct Manifest {
    pub scopes: Vec<ManifestScope>,
}

impl Manifest {
    pub fn scopes(&self) -> Vec<ManifestScope> {
        self.scopes.clone()
    }
}

fn get_server_collections(
    cluster: &RemoteCluster,
    bucket: String,
    scope: String,
    ctrl_c: Arc<AtomicBool>,
    span: Span,
) -> Result<Vec<Collection>, ShellError> {
    let response = cluster
        .cluster()
        .http_client()
        .management_request(
            ManagementRequest::GetCollections { bucket },
            Instant::now().add(cluster.timeouts().management_timeout()),
            ctrl_c.clone(),
        )
        .map_err(|e| client_error_to_shell_error(e, span))?;

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

    Ok(manifest
        .scopes()
        .into_iter()
        .find(|s| s.name == scope)
        .map(|scp| scp.collections())
        .unwrap_or(vec![]))
}
