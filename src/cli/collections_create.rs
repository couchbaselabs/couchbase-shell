//! The `collections get` command fetches all of the collection names from the server.

use crate::cli::util::{
    cluster_identifiers_from, find_org_project_cluster_ids, get_active_cluster,
};
use crate::client::ManagementRequest::CreateCollection;
use crate::state::State;
use log::debug;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

use crate::cli::collections::{get_bucket_or_active, get_scope_or_active};
use crate::cli::error::{
    client_error_to_shell_error, serialize_error, unexpected_status_code_error,
};
use crate::client::cloud::CollectionNamespace;
use crate::remote_cluster::RemoteCluster;
use nu_engine::command_prelude::Call;
use nu_engine::CallExt;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signals, Signature, Span, SyntaxShape};

#[derive(Clone)]
pub struct CollectionsCreate {
    state: Arc<Mutex<State>>,
}

impl CollectionsCreate {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for CollectionsCreate {
    fn name(&self) -> &str {
        "collections create"
    }

    fn signature(&self) -> Signature {
        Signature::build("collections create")
            .required("name", SyntaxShape::String, "the name of the collection")
            .named(
                "bucket",
                SyntaxShape::String,
                "the name of the bucket",
                None,
            )
            .named("scope", SyntaxShape::String, "the name of the scope", None)
            .named(
                "max-expiry",
                SyntaxShape::Int,
                "the maximum expiry for documents in this collection, in seconds",
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

    fn description(&self) -> &str {
        "Creates collections through the HTTP API"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        collections_create(self.state.clone(), engine_state, stack, call, input)
    }
}

fn collections_create(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let signals = engine_state.signals().clone();

    let cluster_identifiers = cluster_identifiers_from(engine_state, stack, &state, call, true)?;
    let guard = state.lock().unwrap();
    let collection: String = call.req(engine_state, stack, 0)?;
    let expiry: i64 = call
        .get_flag(engine_state, stack, "max-expiry")?
        .unwrap_or(0);

    for identifier in cluster_identifiers {
        let active_cluster = get_active_cluster(identifier.clone(), &guard, span)?;

        let bucket = get_bucket_or_active(active_cluster, engine_state, stack, call)?;
        let scope = get_scope_or_active(active_cluster, engine_state, stack, call)?;

        debug!(
            "Running collections create for {:?} on bucket {:?}, scope {:?}",
            &collection, &bucket, &scope
        );

        if active_cluster.is_capella() {
            let client = guard
                .named_or_active_org(active_cluster.capella_org())?
                .client();

            let (org_id, project_id, cluster_id) = find_org_project_cluster_ids(
                &client,
                signals.clone(),
                span,
                identifier,
                guard.named_or_active_project(active_cluster.project())?,
                active_cluster,
            )?;

            let namespace = CollectionNamespace::new(org_id, project_id, cluster_id, bucket, scope);

            client
                .create_collection(collection.clone(), expiry, namespace, signals.clone())
                .map_err(|e| client_error_to_shell_error(e, span))
        } else {
            create_server_collection(
                active_cluster,
                scope.clone(),
                bucket.clone(),
                collection.clone(),
                expiry,
                signals.clone(),
                span,
            )
        }?
    }

    Ok(PipelineData::empty())
}

fn create_server_collection(
    cluster: &RemoteCluster,
    scope: String,
    bucket: String,
    collection: String,
    expiry: i64,
    signals: Signals,
    span: Span,
) -> Result<(), ShellError> {
    let mut form = vec![("name", collection.clone())];
    if expiry > 0 {
        form.push(("maxTTL", expiry.to_string()));
    }

    let form_encoded =
        serde_urlencoded::to_string(&form).map_err(|e| serialize_error(e.to_string(), span))?;

    let response = cluster
        .cluster()
        .http_client()
        .management_request(
            CreateCollection {
                scope,
                bucket,
                payload: form_encoded,
            },
            Instant::now().add(cluster.timeouts().management_timeout()),
            signals.clone(),
        )
        .map_err(|e| client_error_to_shell_error(e, span))?;

    match response.status() {
        200 => Ok(()),
        202 => Ok(()),
        _ => Err(unexpected_status_code_error(
            response.status(),
            response.content()?,
            span,
        )),
    }
}
