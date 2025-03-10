use crate::cli::buckets_builder::{
    BucketSettings, BucketSettingsBuilder, BucketType, DurabilityLevel,
};
use crate::cli::error::{
    client_error_to_shell_error, generic_error, serialize_error, unexpected_status_code_error,
};
use crate::cli::util::{
    cluster_from_conn_str, cluster_identifiers_from, find_org_id, find_project_id,
    get_active_cluster,
};
use crate::client::ManagementRequest;
use crate::remote_cluster::RemoteCluster;
use crate::state::{RemoteCapellaOrganization, State};
use log::debug;
use nu_engine::command_prelude::Call;
use nu_engine::CallExt;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signals, Signature, Span, SyntaxShape};
use std::convert::TryFrom;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::{Duration, Instant};

#[derive(Clone)]
pub struct BucketsCreate {
    state: Arc<Mutex<State>>,
}

impl BucketsCreate {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for BucketsCreate {
    fn name(&self) -> &str {
        "buckets create"
    }

    fn signature(&self) -> Signature {
        Signature::build("buckets create")
            .required("name", SyntaxShape::String, "the name of the bucket")
            .required(
                "ram",
                SyntaxShape::Int,
                "the amount of ram to allocate (mb)",
            )
            .named("type", SyntaxShape::String, "the type of bucket", None)
            .named(
                "replicas",
                SyntaxShape::Int,
                "the number of replicas for the bucket",
                None,
            )
            .switch("flush", "whether to enable flush", None)
            .named(
                "durability",
                SyntaxShape::String,
                "the minimum durability level",
                None,
            )
            .named(
                "expiry",
                SyntaxShape::Int,
                "the maximum expiry for documents created in this bucket (seconds)",
                None,
            )
            .named(
                "clusters",
                SyntaxShape::String,
                "the clusters which should be contacted",
                None,
            )
            .category(Category::Custom("couchbase".to_string()))
    }

    fn description(&self) -> &str {
        "Creates a bucket"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        buckets_create(self.state.clone(), engine_state, stack, call, input)
    }
}

fn buckets_create(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let signals = engine_state.signals().clone();

    let name: String = call.req(engine_state, stack, 0)?;
    let ram: i64 = call.req(engine_state, stack, 1)?;

    let bucket_type: Option<String> = call.get_flag(engine_state, stack, "type")?;
    let replicas: Option<i64> = call.get_flag(engine_state, stack, "replicas")?;
    let flush = call.has_flag(engine_state, stack, "flush")?;
    let durability: Option<String> = call.get_flag(engine_state, stack, "durability")?;
    let expiry: Option<i64> = call.get_flag(engine_state, stack, "expiry")?;
    debug!("Running buckets create for bucket {}", &name);

    let cluster_identifiers = cluster_identifiers_from(engine_state, stack, &state, call, true)?;
    let guard = state.lock().unwrap();

    let mut builder = BucketSettingsBuilder::new(name).ram_quota_mb(ram as u64);
    if let Some(ref t) = bucket_type {
        builder = builder.bucket_type(match BucketType::try_from(t.as_str()) {
            Ok(bt) => bt,
            Err(_e) => {
                return Err(generic_error(
                    format!("Failed to parse bucket type {}", t),
                    "Allow values for bucket type are couchbase, membase, memcached, ephemeral"
                        .to_string(),
                    span,
                ));
            }
        });
    }
    if let Some(r) = replicas {
        builder = builder.num_replicas(match u32::try_from(r) {
            Ok(bt) => bt,
            Err(e) => {
                return Err(generic_error(
                    format!("Failed to parse num replicas {}", e),
                    None,
                    span,
                ));
            }
        });
    }
    if flush {
        builder = builder.flush_enabled(flush);
    }
    if let Some(ref d) = durability {
        builder = builder.minimum_durability_level(match DurabilityLevel::try_from(d.as_str()) {
            Ok(bt) => bt,
            Err(_e) => {
                return Err(generic_error(format!("Failed to parse durability level {}", d),
                                         "Allowed values for durability level are one, majority, majorityAndPersistActive, persistToMajority".to_string(), span));
            }
        });
    }
    if let Some(e) = expiry {
        builder = builder.max_expiry(Duration::from_secs(e as u64));
    }

    let settings = &mut builder.build();
    for identifier in cluster_identifiers {
        let active_cluster = get_active_cluster(identifier.clone(), &guard, span)?;
        settings
            .validate(active_cluster.is_capella())
            .map_err(|e| generic_error("Invalid argument", e.to_string(), span))?;

        if active_cluster.is_capella() {
            let org = guard.named_or_active_org(active_cluster.capella_org())?;

            create_capella_bucket(
                org,
                guard.named_or_active_project(active_cluster.project())?,
                active_cluster,
                identifier.clone(),
                settings,
                signals.clone(),
                span,
            )
        } else {
            let form = settings.as_form();
            let payload = serde_urlencoded::to_string(&form)
                .map_err(|e| serialize_error(e.to_string(), span))?;

            create_server_bucket(payload, active_cluster, span, signals.clone())
        }?;
    }

    Ok(PipelineData::empty())
}

pub fn create_server_bucket(
    payload: String,
    cluster: &RemoteCluster,
    span: Span,
    signals: Signals,
) -> Result<(), ShellError> {
    let response = cluster
        .cluster()
        .http_client()
        .management_request(
            ManagementRequest::CreateBucket { payload },
            Instant::now().add(cluster.timeouts().management_timeout()),
            signals.clone(),
        )
        .map_err(|e| client_error_to_shell_error(e, span))?;

    if response.status() != 202 {
        return Err(unexpected_status_code_error(
            response.status(),
            response.content()?,
            span,
        ));
    }

    Ok(())
}

pub fn create_capella_bucket(
    org: &RemoteCapellaOrganization,
    project: String,
    cluster: &RemoteCluster,
    identifier: String,
    settings: &mut BucketSettings,
    signals: Signals,
    span: Span,
) -> Result<(), ShellError> {
    let client = org.client();

    let org_id = find_org_id(signals.clone(), &client, span)?;
    let project_id = find_project_id(signals.clone(), project, &client, span, org_id.clone())?;
    let json_cluster = cluster_from_conn_str(
        identifier.clone(),
        signals.clone(),
        cluster.hostnames().clone(),
        &client,
        span,
        org_id.clone(),
        project_id.clone(),
    )?;

    // If we leave num_replicas as empty the API always sets the default to 1 which will fail for
    // single node clusters where num_replicas must be zero.
    if settings.num_replicas().is_none() && json_cluster.total_nodes() == 1 {
        settings.set_num_replicas(0)
    }

    let json = settings.as_json();

    client
        .create_bucket(
            org_id,
            project_id,
            json_cluster.id(),
            serde_json::to_string(&json).unwrap(),
            signals,
        )
        .map_err(|e| client_error_to_shell_error(e, span))
}
