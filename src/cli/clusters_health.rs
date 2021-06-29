use crate::cli::cloud_json::JSONCloudClusterHealthResponse;
use crate::cli::util::cluster_identifiers_from;
use crate::client::{CloudRequest, ManagementRequest};
use crate::state::{ClusterTimeouts, RemoteCloud, RemoteCluster, State};
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue, Value};
use nu_source::Tag;
use nu_stream::OutputStream;
use serde::Deserialize;
use std::ops::Add;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

pub struct ClustersHealth {
    state: Arc<Mutex<State>>,
}

impl ClustersHealth {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl nu_engine::WholeStreamCommand for ClustersHealth {
    fn name(&self) -> &str {
        "clusters health"
    }

    fn signature(&self) -> Signature {
        Signature::build("buckets config").named(
            "clusters",
            SyntaxShape::String,
            "the clusters which should be contacted",
            None,
        )
    }

    fn usage(&self) -> &str {
        "Performs health checks on the target cluster(s)"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        health(args, self.state.clone())
    }
}

fn health(args: CommandArgs, state: Arc<Mutex<State>>) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();

    let cluster_identifiers = cluster_identifiers_from(&state, &args, true)?;

    let mut converted = vec![];
    for identifier in cluster_identifiers {
        let guard = state.lock().unwrap();
        let cluster = match guard.clusters().get(&identifier) {
            Some(c) => c,
            None => {
                return Err(ShellError::untagged_runtime_error("Cluster not found"));
            }
        };

        if let Some(c) = cluster.cloud() {
            let cloud = guard.cloud_for_cluster(c)?;
            let values =
                check_cloud_health(&identifier, cloud, cluster.timeouts(), ctrl_c.clone())?;
            for value in values {
                converted.push(value);
            }
        } else {
            converted.push(check_autofailover(&identifier, cluster, ctrl_c.clone())?);

            let bucket_names = grab_bucket_names(cluster, ctrl_c.clone())?;
            for bucket_name in bucket_names {
                converted.push(check_resident_ratio(
                    &bucket_name,
                    &identifier,
                    cluster,
                    ctrl_c.clone(),
                )?);
            }
        }
    }

    Ok(converted.into())
}

fn grab_bucket_names(
    cluster: &RemoteCluster,
    ctrl_c: Arc<AtomicBool>,
) -> Result<Vec<String>, ShellError> {
    let response = cluster.cluster().http_client().management_request(
        ManagementRequest::GetBuckets,
        Instant::now().add(cluster.timeouts().query_timeout()),
        ctrl_c,
    )?;
    let resp: Vec<BucketInfo> = serde_json::from_str(response.content())?;
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
) -> Result<Value, ShellError> {
    let mut collected = TaggedDictBuilder::new(Tag::default());

    let response = cluster.cluster().http_client().management_request(
        ManagementRequest::SettingsAutoFailover,
        Instant::now().add(cluster.timeouts().query_timeout()),
        ctrl_c,
    )?;
    let resp: AutoFailoverSettings = serde_json::from_str(response.content())?;

    collected.insert_value("cluster", identifier.to_string());
    collected.insert_value("check", "Autofailover Enabled".to_string());
    collected.insert_value("bucket", "-".to_string());
    collected.insert_value("expected", UntaggedValue::boolean(true));
    collected.insert_value("actual", UntaggedValue::boolean(resp.enabled));

    let remedy = if resp.enabled {
        "Not needed"
    } else {
        "Enable Autofailover"
    };
    collected.insert_value("remedy", remedy.to_string());

    Ok(collected.into_value())
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
) -> Result<Value, ShellError> {
    let mut collected = TaggedDictBuilder::new(Tag::default());

    let response = cluster.cluster().http_client().management_request(
        ManagementRequest::BucketStats {
            name: bucket_name.to_string(),
        },
        Instant::now().add(cluster.timeouts().query_timeout()),
        ctrl_c,
    )?;
    let resp: BucketStats = serde_json::from_str(response.content())?;
    let ratio = match resp.op.samples.active_resident_ratios.last() {
        Some(r) => *r,
        None => {
            println!("Failed to get resident ratios");
            0
        }
    };

    collected.insert_value("cluster", identifier.to_string());
    collected.insert_value("check", "Resident Ratio Too Low".to_string());
    collected.insert_value("bucket", bucket_name.to_string());
    collected.insert_value("expected", ">= 10%");
    collected.insert_value("actual", format!("{}%", &ratio));

    let remedy = if ratio >= 10 {
        "Not needed"
    } else {
        "Should be more than 10%"
    };
    collected.insert_value("remedy", remedy.to_string());

    Ok(collected.into_value())
}

fn check_cloud_health(
    identifier: &str,
    cloud: &RemoteCloud,
    timeouts: &ClusterTimeouts,
    ctrl_c: Arc<AtomicBool>,
) -> Result<Vec<Value>, ShellError> {
    let mut results = Vec::new();

    let cluster_id = cloud.cloud().find_cluster_id(
        identifier.to_string(),
        Instant::now().add(timeouts.query_timeout()),
        ctrl_c.clone(),
    )?;
    let response = cloud.cloud().cloud_request(
        CloudRequest::GetClusterHealth { cluster_id },
        Instant::now().add(timeouts.query_timeout()),
        ctrl_c,
    )?;
    let resp: JSONCloudClusterHealthResponse = serde_json::from_str(response.content())?;

    let status = resp.status();

    let mut status_collected = TaggedDictBuilder::new(Tag::default());
    status_collected.insert_value("cluster", identifier.to_string());
    status_collected.insert_value("check", "status".to_string());
    status_collected.insert_value("bucket", "-".to_string());
    status_collected.insert_value("expected", "ready".to_string());
    status_collected.insert_value("actual", status.clone());

    let remedy = if status == *"ready" {
        "Not needed"
    } else {
        "Should be ready"
    };
    status_collected.insert_value("remedy", remedy.to_string());

    results.push(status_collected.into_value());

    let health = resp.health();

    let mut health_collected = TaggedDictBuilder::new(Tag::default());
    health_collected.insert_value("cluster", identifier.to_string());
    health_collected.insert_value("check", "health".to_string());
    health_collected.insert_value("bucket", "-".to_string());
    health_collected.insert_value("expected", "healthy".to_string());
    health_collected.insert_value("actual", health.clone());

    let remedy = if health == *"healthy" {
        "Not needed"
    } else {
        "Should be healthy"
    };
    health_collected.insert_value("remedy", remedy.to_string());

    results.push(health_collected.into_value());

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
