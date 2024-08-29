use crate::cli::collections::{get_bucket_or_active, Manifest};
use crate::cli::error::{
    client_error_to_shell_error, deserialize_error, unexpected_status_code_error,
};
use crate::cli::util::{
    cluster_from_conn_str, cluster_identifiers_from, find_org_id, find_project_id,
    get_active_cluster, NuValueMap,
};
use crate::client::ManagementRequest;
use crate::remote_cluster::RemoteCluster;
use crate::remote_cluster::RemoteClusterType::Provisioned;
use crate::state::{RemoteCapellaOrganization, State};
use log::debug;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};
use std::ops::Add;
use std::sync::atomic::AtomicBool;
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

        let scopes = if active_cluster.cluster_type() == Provisioned {
            get_capella_scopes(
                guard.named_or_active_org(active_cluster.capella_org())?,
                guard.named_or_active_project(active_cluster.project())?,
                active_cluster,
                identifier.clone(),
                bucket,
                ctrl_c.clone(),
                span,
            )
        } else {
            get_server_scopes(active_cluster, bucket, ctrl_c.clone(), span)
        }?;

        for scope in scopes {
            let mut collected = NuValueMap::default();
            collected.add_string("scope", scope, span);
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

fn get_capella_scopes(
    org: &RemoteCapellaOrganization,
    project: String,
    cluster: &RemoteCluster,
    identifier: String,
    bucket: String,
    ctrl_c: Arc<AtomicBool>,
    span: Span,
) -> Result<Vec<String>, ShellError> {
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

    let scopes = client
        .list_scopes(
            org_id,
            project_id,
            json_cluster.id(),
            bucket,
            deadline,
            ctrl_c,
        )
        .map_err(|e| client_error_to_shell_error(e, span))?;

    Ok(scopes.scopes().iter().map(|s| s.name().clone()).collect())
}

fn get_server_scopes(
    cluster: &RemoteCluster,
    bucket: String,
    ctrl_c: Arc<AtomicBool>,
    span: Span,
) -> Result<Vec<String>, ShellError> {
    let response = cluster
        .cluster()
        .http_client()
        .management_request(
            ManagementRequest::GetScopes { bucket },
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

    Ok(manifest.scopes.iter().map(|s| s.name.clone()).collect())
}
