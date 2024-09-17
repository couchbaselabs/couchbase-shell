use crate::cli::util::{
    cluster_identifiers_from, find_org_project_cluster_ids, get_active_cluster,
};
use crate::client::ManagementRequest;
use crate::state::State;
use log::debug;
use std::ops::Add;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

use crate::cli::collections::get_bucket_or_active;
use crate::cli::error::{client_error_to_shell_error, unexpected_status_code_error};
use crate::remote_cluster::RemoteCluster;
use crate::remote_cluster::RemoteClusterType::Provisioned;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, Span, SyntaxShape};

#[derive(Clone)]
pub struct ScopesDrop {
    state: Arc<Mutex<State>>,
}

impl ScopesDrop {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for ScopesDrop {
    fn name(&self) -> &str {
        "scopes drop"
    }

    fn signature(&self) -> Signature {
        Signature::build("scopes drop")
            .required("name", SyntaxShape::String, "the name of the scope")
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
        "Deletes scopes through the HTTP API"
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

    let scope: String = call.req(engine_state, stack, 0)?;

    for identifier in cluster_identifiers {
        let active_cluster = get_active_cluster(identifier.clone(), &guard, span)?;

        let bucket = get_bucket_or_active(active_cluster, engine_state, stack, call)?;

        debug!(
            "Running scope drop for {:?} on bucket {:?}",
            &scope, &bucket
        );

        if active_cluster.cluster_type() == Provisioned {
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

            client
                .delete_scope(
                    org_id,
                    project_id,
                    cluster_id,
                    bucket,
                    scope.clone(),
                    ctrl_c.clone(),
                )
                .map_err(|e| client_error_to_shell_error(e, span))
        } else {
            drop_server_scope(
                active_cluster,
                bucket.clone(),
                scope.clone(),
                ctrl_c.clone(),
                span,
            )
        }?;
    }

    Ok(PipelineData::empty())
}

fn drop_server_scope(
    cluster: &RemoteCluster,
    bucket: String,
    scope: String,
    ctrl_c: Arc<AtomicBool>,
    span: Span,
) -> Result<(), ShellError> {
    let response = cluster
        .cluster()
        .http_client()
        .management_request(
            ManagementRequest::DropScope {
                bucket,
                name: scope.clone(),
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
