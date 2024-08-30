//! The `collections drop` commanddrop a collection from the server.

use crate::cli::util::{
    cluster_from_conn_str, cluster_identifiers_from, find_org_id, find_project_id,
    get_active_cluster,
};
use crate::client::ManagementRequest::DropCollection;
use crate::state::{RemoteCapellaOrganization, State};
use log::debug;
use std::ops::Add;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

use crate::cli::collections::{get_bucket_or_active, get_scope_or_active};
use crate::cli::error::{client_error_to_shell_error, unexpected_status_code_error};
use crate::remote_cluster::RemoteCluster;
use crate::remote_cluster::RemoteClusterType::Provisioned;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, Span, SyntaxShape};

#[derive(Clone)]
pub struct CollectionsDrop {
    state: Arc<Mutex<State>>,
}

impl CollectionsDrop {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for CollectionsDrop {
    fn name(&self) -> &str {
        "collections drop"
    }

    fn signature(&self) -> Signature {
        Signature::build("collections drop")
            .required("name", SyntaxShape::String, "the name of the collection")
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
        "Deletes collections through the HTTP API"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        collections_drop(self.state.clone(), engine_state, stack, call, input)
    }
}

fn collections_drop(
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
    let collection: String = call.req(engine_state, stack, 0)?;

    for identifier in cluster_identifiers {
        let active_cluster = get_active_cluster(identifier.clone(), &guard, span)?;

        let bucket = get_bucket_or_active(active_cluster, engine_state, stack, call)?;
        let scope = get_scope_or_active(active_cluster, engine_state, stack, call)?;

        debug!(
            "Running collections drop for {:?} on bucket {:?}, scope {:?}",
            &collection, &bucket, &scope
        );

        if active_cluster.cluster_type() == Provisioned {
            drop_capella_collection(
                guard.named_or_active_org(active_cluster.capella_org())?,
                guard.named_or_active_project(active_cluster.project())?,
                active_cluster,
                bucket.clone(),
                scope.clone(),
                collection.clone(),
                identifier,
                ctrl_c.clone(),
                span,
            )
        } else {
            drop_server_collection(
                active_cluster,
                bucket.clone(),
                scope.clone(),
                collection.clone(),
                span,
                ctrl_c.clone(),
            )
        }?;
    }

    Ok(PipelineData::empty())
}

#[allow(clippy::too_many_arguments)]
fn drop_capella_collection(
    org: &RemoteCapellaOrganization,
    project: String,
    cluster: &RemoteCluster,
    bucket: String,
    scope: String,
    collection: String,
    identifier: String,
    ctrl_c: Arc<AtomicBool>,
    span: Span,
) -> Result<(), ShellError> {
    let client = org.client();
    let deadline = Instant::now().add(org.timeout());

    let org_id = find_org_id(ctrl_c.clone(), &client, deadline, span)?;

    let project_id = find_project_id(
        ctrl_c.clone(),
        project,
        &client,
        deadline,
        span,
        org_id.clone(),
    )?;

    let json_cluster = cluster_from_conn_str(
        identifier.clone(),
        ctrl_c.clone(),
        cluster.hostnames().clone(),
        &client,
        deadline,
        span,
        org_id.clone(),
        project_id.clone(),
    )?;

    client
        .delete_collection(
            org_id,
            project_id,
            json_cluster.id(),
            bucket,
            scope,
            collection,
            deadline,
            ctrl_c,
        )
        .map_err(|e| client_error_to_shell_error(e, span))
}

fn drop_server_collection(
    cluster: &RemoteCluster,
    bucket: String,
    scope: String,
    collection: String,
    span: Span,
    ctrl_c: Arc<AtomicBool>,
) -> Result<(), ShellError> {
    let response = cluster
        .cluster()
        .http_client()
        .management_request(
            DropCollection {
                scope,
                bucket,
                name: collection,
            },
            Instant::now().add(cluster.timeouts().management_timeout()),
            ctrl_c.clone(),
        )
        .map_err(|e| client_error_to_shell_error(e, span))?;

    match response.status() {
        200 => Ok(()),
        _ => {
            return Err(unexpected_status_code_error(
                response.status(),
                response.content(),
                span,
            ));
        }
    }
}
