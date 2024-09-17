use crate::cli::error::client_error_to_shell_error;
use crate::cli::util::{
    cluster_identifiers_from, find_org_project_cluster_ids, get_active_cluster, NuValueMap,
};
use crate::client::{ClientError, ManagementRequest};
use crate::remote_cluster::RemoteCluster;
use crate::remote_cluster::RemoteClusterType::Provisioned;
use crate::state::State;
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
        let active_cluster = get_active_cluster(identifier.clone(), &guard, span)?;

        let result = if active_cluster.cluster_type() == Provisioned {
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
                .load_sample_bucket(
                    org_id,
                    project_id,
                    cluster_id,
                    bucket_name.clone(),
                    ctrl_c.clone(),
                )
                .map_err(|e| client_error_to_shell_error(e, span))
        } else {
            load_sever_sample(active_cluster, bucket_name.clone(), ctrl_c.clone(), span)
        };

        let mut collected = NuValueMap::default();
        collected.add_string("cluster", identifier.clone(), span);
        collected.add_string("sample", bucket_name.clone(), span);

        match result {
            Ok(_) => {
                collected.add_string("status", "success", span);
            }
            Err(e) => {
                collected.add_string("status", format!("failure - {}", e), span);
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

fn load_sever_sample(
    cluster: &RemoteCluster,
    sample: String,
    ctrl_c: Arc<AtomicBool>,
    span: Span,
) -> Result<(), ShellError> {
    let response = cluster
        .cluster()
        .http_client()
        .management_request(
            ManagementRequest::LoadSampleBucket {
                name: format!("[\"{}\"]", sample),
            },
            Instant::now().add(cluster.timeouts().management_timeout()),
            ctrl_c,
        )
        .map_err(|e| client_error_to_shell_error(e, span))?;

    match response.status() {
        202 => Ok(()),
        400 => {
            if response.content().contains("already loaded") {
                Err(ClientError::SampleAlreadyLoaded { sample })
            } else if response.content().contains("not a valid sample") {
                Err(ClientError::InvalidSample { sample })
            } else {
                Err(ClientError::RequestFailed {
                    reason: Some(response.content().into()),
                    key: None,
                })
            }
        }
        _ => Err(ClientError::RequestFailed {
            reason: Some(response.content().into()),
            key: None,
        }),
    }
    .map_err(|e| client_error_to_shell_error(e, span))
}
