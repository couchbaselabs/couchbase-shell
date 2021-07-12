use crate::cli::buckets_builder::{
    BucketSettings, DurabilityLevel, JSONBucketSettings, JSONCloudBucketSettings,
};
use crate::cli::buckets_create::collected_value_from_error_string;
use crate::cli::cloud_json::JSONCloudClusterSummary;
use crate::cli::util::cluster_identifiers_from;
use crate::client::{CloudRequest, HttpResponse, ManagementRequest};
use crate::state::State;
use async_trait::async_trait;
use log::debug;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, Value};
use nu_stream::OutputStream;
use std::convert::TryFrom;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::{Duration, Instant};

pub struct BucketsUpdate {
    state: Arc<Mutex<State>>,
}

impl BucketsUpdate {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for BucketsUpdate {
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
                SyntaxShape::String,
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
    }

    fn usage(&self) -> &str {
        "Updates a bucket"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        buckets_update(self.state.clone(), args)
    }
}

fn buckets_update(state: Arc<Mutex<State>>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();

    let name: String = args.req(0)?;
    let ram = args.get_flag("ram")?;
    let replicas = args.get_flag("replicas")?;
    let flush = args.get_flag("flush")?.unwrap_or(false);
    let durability = args.get_flag("durability")?;
    let expiry = args.get_flag("expiry")?;

    debug!("Running buckets update for bucket {}", &name);

    let cluster_identifiers = cluster_identifiers_from(&state, &args, true)?;
    let guard = state.lock().unwrap();

    let mut results: Vec<Value> = vec![];
    for identifier in cluster_identifiers {
        let active_cluster = match guard.clusters().get(&identifier) {
            Some(c) => c,
            None => {
                results.push(collected_value_from_error_string(
                    identifier.clone(),
                    "Cluster not found",
                ));
                continue;
            }
        };

        if active_cluster.cloud() && (flush || durability.is_some() || expiry.is_some()) {
            results.push(collected_value_from_error_string(
                identifier.clone(),
                "Cloud flag cannot be used with type, flush, durability, or expiry",
            ));
            continue;
        }

        let response: HttpResponse;
        if active_cluster.cloud() {
            let cloud = guard.cloud_control_pane()?.client();

            let deadline = Instant::now().add(active_cluster.timeouts().management_timeout());
            let cluster_response = cloud.cloud_request(
                CloudRequest::GetClusters {},
                deadline.clone(),
                ctrl_c.clone(),
            )?;

            if cluster_response.status() != 200 {
                results.push(collected_value_from_error_string(
                    identifier.clone(),
                    cluster_response.content(),
                ));
                continue;
            }

            let data: serde_json::Value = serde_json::from_str(cluster_response.content())?;
            let v = data.get("data").unwrap().to_string();

            let clusters: Vec<JSONCloudClusterSummary> = serde_json::from_str(v.as_str())?;

            let mut cluster_id: Option<String> = None;
            for c in clusters {
                if c.name() == identifier.clone() {
                    cluster_id = Some(c.id());
                }
            }

            if cluster_id.is_none() {
                results.push(collected_value_from_error_string(
                    identifier.clone(),
                    "Could not find active cluster in cloud control pane",
                ));
                continue;
            }

            let buckets_response = cloud.cloud_request(
                CloudRequest::GetBuckets {
                    cluster_id: cluster_id.clone().unwrap(),
                },
                deadline.clone(),
                ctrl_c.clone(),
            )?;
            if buckets_response.status() != 200 {
                results.push(collected_value_from_error_string(
                    identifier.clone(),
                    buckets_response.content(),
                ));
                continue;
            }

            let mut buckets: Vec<JSONCloudBucketSettings> =
                serde_json::from_str(buckets_response.content())?;

            // Cloud requires that updates are performed on an array of buckets, and we have to include all
            // of the buckets that we want to keep so we need to pull out, change and reinsert the bucket that
            // we want to change.
            let idx = match buckets.iter().position(|b| b.name() == name.clone()) {
                Some(i) => i,
                None => {
                    results.push(collected_value_from_error_string(
                        identifier.clone(),
                        "Bucket not found",
                    ));
                    continue;
                }
            };

            let mut settings = BucketSettings::try_from(buckets.swap_remove(idx))?;
            update_bucket_settings(
                &mut settings,
                ram,
                replicas,
                flush,
                durability.clone(),
                expiry,
            )?;

            buckets.push(JSONCloudBucketSettings::try_from(&settings)?);

            response = cloud.cloud_request(
                CloudRequest::UpdateBucket {
                    cluster_id: cluster_id.unwrap(),
                    payload: serde_json::to_string(&buckets)?,
                },
                deadline.clone(),
                ctrl_c.clone(),
            )?;
        } else {
            let deadline = Instant::now().add(active_cluster.timeouts().management_timeout());
            let get_response = active_cluster.cluster().http_client().management_request(
                ManagementRequest::GetBucket { name: name.clone() },
                deadline.clone(),
                ctrl_c.clone(),
            )?;

            let content: JSONBucketSettings = serde_json::from_str(get_response.content())?;
            let mut settings = BucketSettings::try_from(content)?;

            update_bucket_settings(
                &mut settings,
                ram,
                replicas,
                flush,
                durability.clone(),
                expiry,
            )?;

            let form = settings.as_form(true)?;
            let payload = serde_urlencoded::to_string(&form).unwrap();

            response = active_cluster.cluster().http_client().management_request(
                ManagementRequest::UpdateBucket {
                    name: name.clone(),
                    payload,
                },
                deadline,
                ctrl_c.clone(),
            )?;
        }

        match response.status() {
            200 => {}
            201 => {}
            202 => {}
            _ => {
                results.push(collected_value_from_error_string(
                    identifier.clone(),
                    response.content(),
                ));
            }
        }
    }

    Ok(OutputStream::from(results))
}

fn update_bucket_settings(
    settings: &mut BucketSettings,
    ram: Option<u64>,
    replicas: Option<u64>,
    flush: bool,
    durability: Option<String>,
    expiry: Option<u64>,
) -> Result<(), ShellError> {
    if let Some(r) = ram {
        settings.set_ram_quota_mb(r);
    }
    if let Some(r) = replicas {
        settings.set_num_replicas(match u32::try_from(r) {
            Ok(bt) => bt,
            Err(e) => {
                return Err(ShellError::untagged_runtime_error(format!(
                    "Failed to parse durability level {}",
                    e
                )));
            }
        });
    }
    if flush {
        settings.set_flush_enabled(flush);
    }
    if let Some(d) = durability {
        settings.set_minimum_durability_level(match DurabilityLevel::try_from(d.as_str()) {
            Ok(bt) => bt,
            Err(e) => {
                return Err(ShellError::untagged_runtime_error(format!(
                    "Failed to parse durability level {}",
                    e
                )));
            }
        });
    }
    if let Some(e) = expiry {
        settings.set_max_expiry(Duration::from_secs(e));
    }

    Ok(())
}
