//! The `buckets get` command fetches buckets from the server.
use crate::cli::buckets_get::check_response;
use crate::cli::error::client_error_to_shell_error;
use crate::cli::util::{
    cluster_identifiers_from, find_cluster_id, find_org_id, find_project_id, get_active_cluster,
};
use crate::client::{CapellaRequest, HttpResponse, ManagementRequest};
use crate::remote_cluster::RemoteCluster;
use crate::remote_cluster::RemoteClusterType::Provisioned;
use crate::state::{RemoteCapellaOrganization, State};
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use log::debug;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, Span, SyntaxShape};
use std::ops::Add;
use std::sync::atomic::AtomicBool;
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
    let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();

    let cluster_identifiers = cluster_identifiers_from(engine_state, stack, &state, call, true)?;
    let name: String = call.req(engine_state, stack, 0)?;
    let guard = state.lock().unwrap();

    debug!("Running buckets drop for bucket {:?}", &name);

    for identifier in cluster_identifiers {
        let cluster = get_active_cluster(identifier.clone(), &guard, span)?;

        let response = if cluster.cluster_type() == Provisioned {
            let org = if let Some(cluster_org) = cluster.capella_org() {
                guard.get_capella_org(cluster_org)
            } else {
                guard.active_capella_org()
            }?;

            drop_capella_bucket(
                org,
                guard.active_project()?,
                cluster,
                name.clone(),
                identifier.clone(),
                ctrl_c.clone(),
                span,
            )
        } else {
            drop_server_bucket(cluster, name.clone(), ctrl_c.clone(), span)
        }?;

        check_response(&response, name.clone(), span)?;
    }

    Ok(PipelineData::empty())
}

fn drop_server_bucket(
    cluster: &RemoteCluster,
    name: String,
    ctrl_c: Arc<AtomicBool>,
    span: Span,
) -> Result<HttpResponse, ShellError> {
    cluster
        .cluster()
        .http_client()
        .management_request(
            ManagementRequest::DropBucket { name },
            Instant::now().add(cluster.timeouts().management_timeout()),
            ctrl_c.clone(),
        )
        .map_err(|e| client_error_to_shell_error(e, span))
}

fn drop_capella_bucket(
    org: &RemoteCapellaOrganization,
    project: String,
    cluster: &RemoteCluster,
    bucket: String,
    identifier: String,
    ctrl_c: Arc<AtomicBool>,
    span: Span,
) -> Result<HttpResponse, ShellError> {
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

    let cluster_id = find_cluster_id(
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
        .capella_request(
            CapellaRequest::DropBucketV4 {
                org_id,
                project_id,
                cluster_id,
                bucket_id: BASE64_STANDARD.encode(bucket.clone()),
            },
            deadline,
            ctrl_c.clone(),
        )
        .map_err(|e| client_error_to_shell_error(e, span))
}
