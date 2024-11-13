//! The `buckets get` command fetches buckets from the server.
use crate::cli::error::client_error_to_shell_error;
use crate::cli::util::{
    cluster_identifiers_from, find_org_project_cluster_ids, get_active_cluster,
};
use crate::client::{ClientError, ManagementRequest};
use crate::remote_cluster::RemoteCluster;
use crate::remote_cluster::RemoteClusterType::Provisioned;
use crate::state::State;
use log::debug;
use nu_engine::command_prelude::Call;
use nu_engine::CallExt;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signals, Signature, Span, SyntaxShape};
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

#[derive(Clone)]
pub struct BucketsDrop {
    state: Arc<Mutex<State>>,
}

impl BucketsDrop {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for BucketsDrop {
    fn name(&self) -> &str {
        "buckets drop"
    }

    fn signature(&self) -> Signature {
        Signature::build("buckets drop")
            .required("name", SyntaxShape::String, "the name of the bucket")
            .named(
                "clusters",
                SyntaxShape::String,
                "the clusters which should be contacted",
                None,
            )
            .category(Category::Custom("couchbase".to_string()))
    }

    fn usage(&self) -> &str {
        "Drops buckets through the HTTP API"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        buckets_drop(self.state.clone(), engine_state, stack, call, input)
    }
}

fn buckets_drop(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let signals = engine_state.signals().clone();

    let cluster_identifiers = cluster_identifiers_from(engine_state, stack, &state, call, true)?;
    let name: String = call.req(engine_state, stack, 0)?;
    let guard = state.lock().unwrap();

    debug!("Running buckets drop for bucket {:?}", &name);

    for identifier in cluster_identifiers {
        let active_cluster = get_active_cluster(identifier.clone(), &guard, span)?;

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
                .delete_bucket(
                    org_id,
                    project_id,
                    cluster_id,
                    name.clone(),
                    signals.clone(),
                )
                .map_err(|e| client_error_to_shell_error(e, span))
        } else {
            drop_server_bucket(active_cluster, name.clone(), signals.clone(), span)
        }?;
    }

    Ok(PipelineData::empty())
}

fn drop_server_bucket(
    cluster: &RemoteCluster,
    name: String,
    signals: Signals,
    span: Span,
) -> Result<(), ShellError> {
    let response = cluster
        .cluster()
        .http_client()
        .management_request(
            ManagementRequest::DropBucket { name },
            Instant::now().add(cluster.timeouts().management_timeout()),
            signals.clone(),
        )
        .map_err(|e| client_error_to_shell_error(e, span))?;

    if response.status() != 200 {
        Err(ClientError::RequestFailed {
            reason: Some(response.content()?),
            key: None,
        })
        .map_err(|e| client_error_to_shell_error(e, span))?;
    }
    Ok(())
}
