use crate::cli::error::client_error_to_shell_error;
use crate::cli::util::{
    cluster_identifiers_from, find_cluster_id, find_org_id, find_project_id, get_active_cluster,
    NuValueMap,
};
use crate::client::{CapellaRequest, HttpResponse, ManagementRequest};
use crate::remote_cluster::RemoteCluster;
use crate::remote_cluster::RemoteClusterType::Provisioned;
use crate::state::{RemoteCapellaOrganization, State};
use nu_engine::CallExt;
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
pub struct BucketsSample {
    state: Arc<Mutex<State>>,
}

impl BucketsSample {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for BucketsSample {
    fn name(&self) -> &str {
        "buckets load-sample"
    }

    fn signature(&self) -> Signature {
        Signature::build("buckets load-sample")
            .required(
                "name",
                SyntaxShape::String,
                "the name of the bucket to load",
            )
            .named(
                "clusters",
                SyntaxShape::String,
                "the clusters which should be contacted",
                None,
            )
            .category(Category::Custom("couchbase".to_string()))
    }

    fn usage(&self) -> &str {
        "Load a sample bucket"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        load_sample_bucket(self.state.clone(), engine_state, stack, call, input)
    }
}

fn load_sample_bucket(
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
    let bucket_name: String = call.req(engine_state, stack, 0)?;

    let mut results: Vec<Value> = vec![];
    for identifier in cluster_identifiers {
        let cluster = get_active_cluster(identifier.clone(), &guard, span)?;

        let response = if cluster.cluster_type() == Provisioned {
            let org = if let Some(cluster_org) = cluster.capella_org() {
                guard.get_capella_org(cluster_org)
            } else {
                guard.active_capella_org()
            }?;

            load_capella_sample(
                org,
                guard.active_project()?,
                cluster,
                identifier.clone(),
                format!("{{\"name\": \"{}\"}}", bucket_name.clone()),
                ctrl_c.clone(),
                span,
            )
        } else {
            load_sever_sample(cluster, bucket_name.clone(), ctrl_c.clone(), span)
        }?;

        let mut collected = NuValueMap::default();
        collected.add_string("cluster", identifier.clone(), span);
        collected.add_string("sample", bucket_name.clone(), span);

        match response.status() {
            201 | 202 => {
                collected.add_string("status", "success", span);
            }
            // Couchbase server returns 400 for invalid/not-found and already loaded sample
            400 => {
                if response.content().contains("already loaded") {
                    collected.add_string(
                        "status",
                        format!(
                            "failure - Sample bucket {} is already loaded.",
                            bucket_name.clone()
                        ),
                        span,
                    );
                } else if response.content().contains("not a valid sample") {
                    collected.add_string(
                        "status",
                        format!(
                            "failure - Sample {} is not a valid sample.",
                            bucket_name.clone()
                        ),
                        span,
                    );
                } else {
                    collected.add_string(
                        "status",
                        format!(
                            "failure - unexpected error occurred: {}",
                            response.content()
                        ),
                        span,
                    );
                }
            }
            // Capella v4 API returns 422 for invalid/not-found sample
            422 => {
                collected.add_string(
                    "status",
                    format!(
                        "failure - Sample {} is not a valid sample.",
                        bucket_name.clone()
                    ),
                    span,
                );
            }
            _ => {
                collected.add_string(
                    "status",
                    format!(
                        "failure - unexpected error occurred: {}",
                        response.content()
                    ),
                    span,
                );
            }
        }

        results.push(collected.into_value(span));
    }

    Ok(Value::List {
        vals: results,
        internal_span: span,
    }
    .into_pipeline_data())
}

fn load_capella_sample(
    org: &RemoteCapellaOrganization,
    project: String,
    cluster: &RemoteCluster,
    identifier: String,
    payload: String,
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
            CapellaRequest::LoadSampleBucketV4 {
                org_id,
                project_id,
                cluster_id,
                payload,
            },
            deadline,
            ctrl_c.clone(),
        )
        .map_err(|e| client_error_to_shell_error(e, span))
}

fn load_sever_sample(
    cluster: &RemoteCluster,
    name: String,
    ctrl_c: Arc<AtomicBool>,
    span: Span,
) -> Result<HttpResponse, ShellError> {
    cluster
        .cluster()
        .http_client()
        .management_request(
            ManagementRequest::LoadSampleBucket {
                name: format!("[\"{}\"]", name),
            },
            Instant::now().add(cluster.timeouts().management_timeout()),
            ctrl_c,
        )
        .map_err(|e| client_error_to_shell_error(e, span))
}
