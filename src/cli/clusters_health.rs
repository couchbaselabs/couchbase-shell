use crate::cli::cloud_json::JSONCloudClusterHealthResponse;
use crate::cli::util::{cluster_identifiers_from, get_active_cluster, NuValueMap};
use crate::client::{CapellaRequest, ManagementRequest};
use crate::state::{
    CapellaEnvironment, ClusterTimeouts, RemoteCapellaOrganization, RemoteCluster, State,
};
use log::warn;
use serde::Deserialize;
use std::ops::Add;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

use crate::cli::error::{cant_run_against_hosted_capella_error, deserialize_error};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct ClustersHealth {
    state: Arc<Mutex<State>>,
}

impl ClustersHealth {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for ClustersHealth {
    fn name(&self) -> &str {
        "clusters health"
    }

    fn signature(&self) -> Signature {
        Signature::build("clusters health")
            .named(
                "clusters",
                SyntaxShape::String,
                "the clusters which should be contacted",
                None,
            )
            .category(Category::Custom("couchbase".into()))
    }

    fn usage(&self) -> &str {
        "Performs health checks on the target cluster(s)"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        health(self.state.clone(), engine_state, stack, call, input)
    }
}

fn health(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();

    let cluster_identifiers = cluster_identifiers_from(&engine_state, stack, &state, &call, true)?;

    let mut converted = vec![];
    for identifier in cluster_identifiers {
        let guard = state.lock().unwrap();
        let cluster = get_active_cluster(identifier.clone(), &guard, span.clone())?;

        if let Some(plane) = cluster.capella_org() {
            let cloud = guard.capella_org_for_cluster(plane)?;
            let values =
                check_cloud_health(&identifier, cloud, cluster.timeouts(), ctrl_c.clone(), span)?;
            for value in values {
                converted.push(value);
            }
        } else {
            converted.push(check_autofailover(
                &identifier,
                cluster,
                ctrl_c.clone(),
                span,
            )?);

            let bucket_names = grab_bucket_names(cluster, ctrl_c.clone(), span.clone())?;
            for bucket_name in bucket_names {
                converted.push(check_resident_ratio(
                    &bucket_name,
                    &identifier,
                    cluster,
                    ctrl_c.clone(),
                    span,
                )?);
            }
        }
    }

    Ok(Value::List {
        vals: converted,
        span,
    }
    .into_pipeline_data())
}

fn grab_bucket_names(
    cluster: &RemoteCluster,
    ctrl_c: Arc<AtomicBool>,
    span: Span,
) -> Result<Vec<String>, ShellError> {
    let response = cluster.cluster().http_client().management_request(
        ManagementRequest::GetBuckets,
        Instant::now().add(cluster.timeouts().management_timeout()),
        ctrl_c,
    )?;
    let resp: Vec<BucketInfo> = serde_json::from_str(response.content())
        .map_err(|e| deserialize_error(e.to_string(), span))?;
    Ok(resp.into_iter().map(|b| b.name).collect::<Vec<_>>())
}

#[derive(Debug, Deserialize)]
struct BucketInfo {
    name: String,
}

fn check_autofailover(
    identifier: &str,
    cluster: &RemoteCluster,
    ctrl_c: Arc<AtomicBool>,
    span: Span,
) -> Result<Value, ShellError> {
    let response = cluster.cluster().http_client().management_request(
        ManagementRequest::SettingsAutoFailover,
        Instant::now().add(cluster.timeouts().management_timeout()),
        ctrl_c,
    )?;
    let resp: AutoFailoverSettings = serde_json::from_str(response.content())
        .map_err(|e| deserialize_error(e.to_string(), span))?;

    let mut collected = NuValueMap::default();
    collected.add_string("cluster", identifier.to_string(), span);
    collected.add_string("check", "Autofailover Enabled".to_string(), span);
    collected.add_string("bucket", "-".to_string(), span);
    collected.add_bool("expected", true, span);
    collected.add_bool("actual", resp.enabled, span);
    collected.add_bool("capella", false, span);

    let remedy = if resp.enabled {
        "Not needed"
    } else {
        "Enable Autofailover"
    };
    collected.add_string("remedy", remedy.to_string(), span);

    Ok(collected.into_value(span))
}

#[derive(Debug, Deserialize)]
struct AutoFailoverSettings {
    enabled: bool,
}

fn check_resident_ratio(
    bucket_name: &str,
    identifier: &str,
    cluster: &RemoteCluster,
    ctrl_c: Arc<AtomicBool>,
    span: Span,
) -> Result<Value, ShellError> {
    let response = cluster.cluster().http_client().management_request(
        ManagementRequest::BucketStats {
            name: bucket_name.to_string(),
        },
        Instant::now().add(cluster.timeouts().management_timeout()),
        ctrl_c,
    )?;
    let resp: BucketStats = serde_json::from_str(response.content())
        .map_err(|e| deserialize_error(e.to_string(), span))?;
    let ratio = match resp.op.samples.active_resident_ratios.last() {
        Some(r) => *r,
        None => {
            warn!("Failed to get resident ratios");
            0
        }
    };

    let mut collected = NuValueMap::default();
    collected.add_string("cluster", identifier.to_string(), span);
    collected.add_string("check", "Resident Ratio Too Low".to_string(), span);
    collected.add_string("bucket", bucket_name.to_string(), span);
    collected.add_string("expected", ">= 10%", span);
    collected.add_string("actual", format!("{}%", &ratio), span);
    collected.add_bool("capella", false, span);

    let remedy = if ratio >= 10 {
        "Not needed"
    } else {
        "Should be more than 10%"
    };
    collected.add_string("remedy", remedy.to_string(), span);

    Ok(collected.into_value(span))
}

fn check_cloud_health(
    identifier: &str,
    cloud: &RemoteCapellaOrganization,
    timeouts: ClusterTimeouts,
    ctrl_c: Arc<AtomicBool>,
    span: Span,
) -> Result<Vec<Value>, ShellError> {
    let mut results = Vec::new();

    let deadline = Instant::now().add(timeouts.management_timeout());
    let cluster =
        cloud
            .client()
            .find_cluster(identifier.to_string(), deadline.clone(), ctrl_c.clone())?;

    if cluster.environment() == CapellaEnvironment::Hosted {
        return Err(cant_run_against_hosted_capella_error(
            "clusters health",
            span,
        ));
    }

    let response = cloud.client().capella_request(
        CapellaRequest::GetClusterHealth {
            cluster_id: cluster.id(),
        },
        deadline,
        ctrl_c,
    )?;
    let resp: JSONCloudClusterHealthResponse = serde_json::from_str(response.content())
        .map_err(|e| deserialize_error(e.to_string(), span))?;

    let status = resp.status();

    let mut status_collected = NuValueMap::default();
    status_collected.add_string("cluster", identifier.to_string(), span);
    status_collected.add_string("check", "Status".to_string(), span);
    status_collected.add_string("bucket", "-".to_string(), span);
    status_collected.add_string("expected", "ready".to_string(), span);
    status_collected.add_string("actual", status.clone(), span);
    status_collected.add_bool("capella", true, span);

    let remedy = if status == *"ready" {
        "Not needed"
    } else {
        "Should be ready"
    };
    status_collected.add_string("remedy", remedy.to_string(), span);

    results.push(status_collected.into_value(span));

    let health = resp.health();

    let mut health_collected = NuValueMap::default();
    health_collected.add_string("cluster", identifier.to_string(), span);
    health_collected.add_string("check", "Health".to_string(), span);
    health_collected.add_string("bucket", "-".to_string(), span);
    health_collected.add_string("expected", "healthy".to_string(), span);
    health_collected.add_string("actual", health.clone(), span);
    health_collected.add_bool("capella", true, span);

    let remedy = if health == *"healthy" {
        "Not needed"
    } else {
        "Should be healthy"
    };
    health_collected.add_string("remedy", remedy.to_string(), span);

    results.push(health_collected.into_value(span));

    Ok(results)
}

#[derive(Debug, Deserialize)]
struct BucketStats {
    op: BucketStatsOp,
}

#[derive(Debug, Deserialize)]
struct BucketStatsOp {
    samples: BucketStatsSamples,
}

#[derive(Debug, Deserialize)]
struct BucketStatsSamples {
    #[serde(rename = "vb_active_resident_items_ratio")]
    active_resident_ratios: Vec<u32>,
}
