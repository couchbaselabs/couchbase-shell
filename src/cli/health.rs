use crate::cli::error::{
    client_error_to_shell_error, deserialize_error, unexpected_status_code_error,
};
use crate::cli::util::{
    cluster_identifiers_from, get_active_cluster, validate_is_not_cloud, NuValueMap,
};
use crate::client::ManagementRequest;
use crate::state::State;
use crate::RemoteCluster;
use log::warn;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};
use serde::Deserialize;
use std::ops::Add;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

#[derive(Clone)]
pub struct HealthCheck {
    state: Arc<Mutex<State>>,
}

impl HealthCheck {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for HealthCheck {
    fn name(&self) -> &str {
        "health"
    }

    fn signature(&self) -> Signature {
        Signature::build("health")
            .named(
                "clusters",
                SyntaxShape::String,
                "the clusters which should be contacted",
                None,
            )
            .category(Category::Custom("couchbase".to_string()))
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

    let cluster_identifiers = cluster_identifiers_from(engine_state, stack, &state, call, true)?;

    let mut converted = vec![];
    for identifier in cluster_identifiers {
        let guard = state.lock().unwrap();
        let cluster = get_active_cluster(identifier.clone(), &guard, span)?;
        validate_is_not_cloud(cluster, "clusters health", span)?;

        converted.push(check_autofailover(
            &identifier,
            cluster,
            ctrl_c.clone(),
            span,
        )?);

        let bucket_names = grab_bucket_names(cluster, ctrl_c.clone(), span)?;
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

    Ok(Value::List {
        vals: converted,
        internal_span: span,
    }
    .into_pipeline_data())
}

fn grab_bucket_names(
    cluster: &RemoteCluster,
    ctrl_c: Arc<AtomicBool>,
    span: Span,
) -> Result<Vec<String>, ShellError> {
    let response = cluster
        .cluster()
        .http_client()
        .management_request(
            ManagementRequest::GetBuckets,
            Instant::now().add(cluster.timeouts().management_timeout()),
            ctrl_c,
        )
        .map_err(|e| client_error_to_shell_error(e, span))?;
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
    let response = cluster
        .cluster()
        .http_client()
        .management_request(
            ManagementRequest::SettingsAutoFailover,
            Instant::now().add(cluster.timeouts().management_timeout()),
            ctrl_c,
        )
        .map_err(|e| client_error_to_shell_error(e, span))?;
    if response.status() != 200 {
        return Err(unexpected_status_code_error(
            response.status(),
            response.content(),
            span,
        ));
    };
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
    let response = cluster
        .cluster()
        .http_client()
        .management_request(
            ManagementRequest::BucketStats {
                name: bucket_name.to_string(),
            },
            Instant::now().add(cluster.timeouts().management_timeout()),
            ctrl_c,
        )
        .map_err(|e| client_error_to_shell_error(e, span))?;
    if response.status() != 200 {
        return Err(unexpected_status_code_error(
            response.status(),
            response.content(),
            span,
        ));
    };
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
