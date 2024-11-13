use crate::cli::util::{
    cluster_identifiers_from, find_org_project_cluster_ids, get_active_cluster,
};
use crate::client::ManagementRequest;
use crate::state::State;
use log::debug;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

use crate::cli::collections::get_bucket_or_active;
use crate::cli::error::{
    client_error_to_shell_error, serialize_error, unexpected_status_code_error,
};
use crate::remote_cluster::RemoteCluster;
use crate::remote_cluster::RemoteClusterType::Provisioned;
use nu_engine::command_prelude::Call;
use nu_engine::CallExt;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signals, Signature, Span, SyntaxShape};

#[derive(Clone)]
pub struct ScopesCreate {
    state: Arc<Mutex<State>>,
}

impl ScopesCreate {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for ScopesCreate {
    fn name(&self) -> &str {
        "scopes create"
    }

    fn signature(&self) -> Signature {
        Signature::build("scopes create")
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

    fn description(&self) -> &str {
        "Creates scopes through the HTTP API"
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
    let signals = engine_state.signals().clone();

    let cluster_identifiers = cluster_identifiers_from(engine_state, stack, &state, call, true)?;
    let guard = state.lock().unwrap();

    let scope: String = call.req(engine_state, stack, 0)?;

    for identifier in cluster_identifiers {
        let active_cluster = get_active_cluster(identifier.clone(), &guard, span)?;

        let bucket = get_bucket_or_active(active_cluster, engine_state, stack, call)?;

        debug!(
            "Running scope create for {:?} on bucket {:?}",
            &scope, &bucket
        );

        if active_cluster.cluster_type() == Provisioned {
            let client = guard
                .named_or_active_org(active_cluster.capella_org())?
                .client();

            let (org_id, project_id, cluster_id) = find_org_project_cluster_ids(
                &client,
                signals.clone(),
                span,
                identifier.clone(),
                guard.named_or_active_project(active_cluster.project())?,
                active_cluster,
            )?;

            client
                .create_scope(
                    org_id,
                    project_id,
                    cluster_id,
                    bucket,
                    scope.clone(),
                    signals.clone(),
                )
                .map_err(|e| client_error_to_shell_error(e, span))
        } else {
            create_server_scope(
                active_cluster,
                bucket.clone(),
                scope.clone(),
                signals.clone(),
                span,
            )
        }?;
    }

    Ok(PipelineData::empty())
}

fn create_server_scope(
    cluster: &RemoteCluster,
    bucket: String,
    scope: String,
    signals: Signals,
    span: Span,
) -> Result<(), ShellError> {
    let form = vec![("name", scope.clone())];
    let payload =
        serde_urlencoded::to_string(form).map_err(|e| serialize_error(e.to_string(), span))?;
    let response = cluster
        .cluster()
        .http_client()
        .management_request(
            ManagementRequest::CreateScope { payload, bucket },
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
