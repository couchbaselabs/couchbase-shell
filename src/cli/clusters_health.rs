use crate::cli::util::cluster_identifiers_from;
use crate::client::ManagementRequest;
use crate::state::State;
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
    let args = args.evaluate_once()?;

    let cluster_identifiers = cluster_identifiers_from(&state, &args, true)?;

    let mut converted = vec![];
    for identifier in cluster_identifiers {
        converted.push(check_autofailover(
            state.clone(),
            &identifier,
            ctrl_c.clone(),
        )?);

        let bucket_names = grab_bucket_names(state.clone(), &identifier, ctrl_c.clone())?;
        for bucket_name in bucket_names {
            converted.push(check_resident_ratio(
                state.clone(),
                &bucket_name,
                &identifier,
                ctrl_c.clone(),
            )?);
        }
    }

    Ok(converted.into())
}

fn grab_bucket_names(
    state: Arc<Mutex<State>>,
    identifier: &str,
    ctrl_c: Arc<AtomicBool>,
) -> Result<Vec<String>, ShellError> {
    let guard = state.lock().unwrap();
    let cluster = match guard.clusters().get(identifier) {
        Some(c) => c,
        None => {
            return Err(ShellError::untagged_runtime_error("Cluster not found"));
        }
    };

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
    state: Arc<Mutex<State>>,
    identifier: &str,
    ctrl_c: Arc<AtomicBool>,
) -> Result<Value, ShellError> {
    let mut collected = TaggedDictBuilder::new(Tag::default());

    let guard = state.lock().unwrap();
    let cluster = match guard.clusters().get(identifier) {
        Some(c) => c,
        None => {
            return Err(ShellError::untagged_runtime_error("Cluster not found"));
        }
    };

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
    state: Arc<Mutex<State>>,
    bucket_name: &str,
    identifier: &str,
    ctrl_c: Arc<AtomicBool>,
) -> Result<Value, ShellError> {
    let mut collected = TaggedDictBuilder::new(Tag::default());

    let guard = state.lock().unwrap();
    let cluster = match guard.clusters().get(identifier) {
        Some(c) => c,
        None => {
            return Err(ShellError::untagged_runtime_error("Cluster not found"));
        }
    };

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
