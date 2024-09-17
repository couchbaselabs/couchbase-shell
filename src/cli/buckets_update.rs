use crate::cli::buckets_builder::{BucketSettings, DurabilityLevel};
use crate::cli::buckets_get::{get_capella_bucket, get_server_bucket};
use crate::cli::error::{client_error_to_shell_error, generic_error, serialize_error};
use crate::cli::unexpected_status_code_error;
use crate::cli::util::{
    cluster_identifiers_from, find_org_project_cluster_ids, get_active_cluster,
};
use crate::client::ManagementRequest;
use crate::remote_cluster::RemoteCluster;
use crate::remote_cluster::RemoteClusterType::Provisioned;
use crate::state::State;
use log::debug;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, Span, SyntaxShape};
use std::convert::TryFrom;
use std::ops::Add;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use tokio::time::{Duration, Instant};

#[derive(Clone)]
pub struct BucketsUpdate {
    state: Arc<Mutex<State>>,
}

impl BucketsUpdate {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for BucketsUpdate {
    fn name(&self) -> &str {
        "buckets update"
    }

    fn signature(&self) -> Signature {
        Signature::build("buckets update")
            .required("name", SyntaxShape::String, "the name of the bucket")
            .named(
                "ram",
                SyntaxShape::Int,
                "the amount of ram to allocate (mb)",
                None,
            )
            .named(
                "replicas",
                SyntaxShape::Int,
                "the number of replicas for the bucket",
                None,
            )
            .named(
                "flush",
                SyntaxShape::Boolean,
                "whether to enable flush",
                None,
            )
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

    fn usage(&self) -> &str {
        "Updates a bucket"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        buckets_update(self.state.clone(), engine_state, stack, call, input)
    }
}

fn buckets_update(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();

    let name: String = call.req(engine_state, stack, 0)?;
    let ram: Option<i64> = call.get_flag(engine_state, stack, "ram")?;
    let replicas: Option<i64> = call.get_flag(engine_state, stack, "replicas")?;
    let flush = call.get_flag(engine_state, stack, "flush")?;
    let durability = call.get_flag(engine_state, stack, "durability")?;
    let expiry: Option<i64> = call.get_flag(engine_state, stack, "expiry")?;

    debug!("Running buckets update for bucket {}", &name);

    let cluster_identifiers = cluster_identifiers_from(engine_state, stack, &state, call, true)?;
    let guard = state.lock().unwrap();

    for identifier in cluster_identifiers {
        let active_cluster = get_active_cluster(identifier.clone(), &guard, span)?;

        let mut settings = if active_cluster.cluster_type() == Provisioned {
            let org = guard.named_or_active_org(active_cluster.capella_org())?;

            get_capella_bucket(
                org,
                guard.named_or_active_project(active_cluster.project())?,
                active_cluster,
                name.clone(),
                identifier.clone(),
                ctrl_c.clone(),
                span,
            )
        } else {
            get_server_bucket(active_cluster, name.clone(), ctrl_c.clone(), span)
        }?;

        update_bucket_settings(
            &mut settings,
            ram.map(|v| v as u64),
            replicas.map(|v| v as u64),
            flush,
            durability.clone(),
            expiry.map(|v| v as u64),
            span,
        )?;

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

            let json = settings.as_json();

            client
                .update_bucket(
                    org_id,
                    project_id,
                    cluster_id,
                    settings.name().into(),
                    serde_json::to_string(&json).unwrap(),
                    ctrl_c.clone(),
                )
                .map_err(|e| client_error_to_shell_error(e, span))
        } else {
            update_server_bucket(settings, active_cluster, ctrl_c.clone(), span)
        }?;
    }

    Ok(PipelineData::empty())
}

fn update_server_bucket(
    settings: BucketSettings,
    cluster: &RemoteCluster,
    ctrl_c: Arc<AtomicBool>,
    span: Span,
) -> Result<(), ShellError> {
    let form = settings.as_form();
    let payload =
        serde_urlencoded::to_string(form).map_err(|e| serialize_error(e.to_string(), span))?;

    let response = cluster
        .cluster()
        .http_client()
        .management_request(
            ManagementRequest::UpdateBucket {
                name: settings.name().to_string(),
                payload,
            },
            Instant::now().add(cluster.timeouts().management_timeout()),
            ctrl_c.clone(),
        )
        .map_err(|e| client_error_to_shell_error(e, span))?;

    if response.status() != 200 {
        return Err(unexpected_status_code_error(
            response.status(),
            response.content(),
            span,
        ));
    }
    Ok(())
}

fn update_bucket_settings(
    settings: &mut BucketSettings,
    ram: Option<u64>,
    replicas: Option<u64>,
    flush: Option<bool>,
    durability: Option<String>,
    expiry: Option<u64>,
    span: Span,
) -> Result<(), ShellError> {
    if let Some(r) = ram {
        settings.set_ram_quota_mb(r);
    }
    if let Some(r) = replicas {
        settings.set_num_replicas(match u32::try_from(r) {
            Ok(bt) => bt,
            Err(e) => {
                return Err(generic_error(
                    format!("Failed to parse num replicas {}", e),
                    "Num replicas must be an unsigned 32 bit integer".to_string(),
                    span,
                ));
            }
        });
    }
    if let Some(f) = flush {
        settings.set_flush_enabled(f);
    }
    if let Some(d) = durability {
        settings.set_minimum_durability_level(match DurabilityLevel::try_from(d.as_str()) {
            Ok(bt) => bt,
            Err(_e) => {

                return Err(generic_error(format!("Failed to parse durability level {}", d),
                                         "Allowed values for durability level are one, majority, majorityAndPersistActive, persistToMajority".to_string(), span));
            }
        });
    }
    if let Some(e) = expiry {
        settings.set_max_expiry(Duration::from_secs(e));
    }

    Ok(())
}
