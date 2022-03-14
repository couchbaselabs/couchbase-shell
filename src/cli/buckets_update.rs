use crate::cli::buckets_builder::{
    BucketSettings, DurabilityLevel, JSONBucketSettings, JSONCloudBucketSettings,
};
use crate::cli::util::{
    cant_run_against_hosted_capella_error, cluster_identifiers_from, cluster_not_found_error,
    generic_labeled_error, map_serde_deserialize_error_to_shell_error,
    map_serde_serialize_error_to_shell_error,
};
use crate::client::{CapellaRequest, HttpResponse, ManagementRequest};
use crate::state::{CapellaEnvironment, State};
use log::debug;
use std::convert::TryFrom;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::{Duration, Instant};

use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape};

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
            .category(Category::Custom("couchbase".into()))
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
    let flush = call
        .get_flag(engine_state, stack, "flush")?
        .unwrap_or(false);
    let durability = call.get_flag(engine_state, stack, "durability")?;
    let expiry: Option<i64> = call.get_flag(engine_state, stack, "expiry")?;

    debug!("Running buckets update for bucket {}", &name);

    let cluster_identifiers = cluster_identifiers_from(&engine_state, stack, &state, &call, true)?;
    let guard = state.lock().unwrap();

    for identifier in cluster_identifiers {
        let active_cluster = match guard.clusters().get(&identifier) {
            Some(c) => c,
            None => {
                return Err(cluster_not_found_error(identifier));
            }
        };

        if active_cluster.capella_org().is_some()
            && (flush || durability.is_some() || expiry.is_some())
        {
            return Err(generic_labeled_error(
                "Capella flag cannot be used with type, flush, durability, or expiry",
                "Capella flag cannot be used with type, flush, durability, or expiry",
            ));
        }

        let response: HttpResponse;
        if let Some(plane) = active_cluster.capella_org() {
            let cloud = guard.capella_org_for_cluster(plane)?.client();

            let deadline = Instant::now().add(active_cluster.timeouts().management_timeout());
            let cluster = cloud.find_cluster(identifier.clone(), deadline, ctrl_c.clone())?;

            if cluster.environment() == CapellaEnvironment::Hosted {
                return Err(cant_run_against_hosted_capella_error());
            }

            let buckets_response = cloud.capella_request(
                CapellaRequest::GetBuckets {
                    cluster_id: cluster.id(),
                },
                deadline.clone(),
                ctrl_c.clone(),
            )?;
            if buckets_response.status() != 200 {
                return Err(generic_labeled_error(
                    "Failed to get buckets",
                    format!("Failed to get buckets: {}", buckets_response.content()),
                ));
            }

            let mut buckets: Vec<JSONCloudBucketSettings> =
                serde_json::from_str(buckets_response.content())
                    .map_err(map_serde_deserialize_error_to_shell_error)?;

            // Cloud requires that updates are performed on an array of buckets, and we have to include all
            // of the buckets that we want to keep so we need to pull out, change and reinsert the bucket that
            // we want to change.
            let idx = match buckets.iter().position(|b| b.name() == name.clone()) {
                Some(i) => i,
                None => {
                    return Err(ShellError::LabeledError(
                        "Bucket not found".into(),
                        format!("Bucket named {} is not known", name),
                    ));
                }
            };

            let mut settings = BucketSettings::try_from(buckets.swap_remove(idx))?;
            update_bucket_settings(
                &mut settings,
                ram.map(|v| v as u64),
                replicas.map(|v| v as u64),
                flush,
                durability.clone(),
                expiry.map(|v| v as u64),
            )?;

            buckets.push(JSONCloudBucketSettings::try_from(&settings)?);

            response = cloud.capella_request(
                CapellaRequest::UpdateBucket {
                    cluster_id: cluster.id(),
                    payload: serde_json::to_string(&buckets)
                        .map_err(map_serde_serialize_error_to_shell_error)?,
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

            let content: JSONBucketSettings = serde_json::from_str(get_response.content())
                .map_err(map_serde_deserialize_error_to_shell_error)?;
            let mut settings = BucketSettings::try_from(content)?;

            update_bucket_settings(
                &mut settings,
                ram.map(|v| v as u64),
                replicas.map(|v| v as u64),
                flush,
                durability.clone(),
                expiry.map(|v| v as u64),
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
                return Err(generic_labeled_error(
                    "Failed to update bucket",
                    format!("Failed to update bucket: {}", response.content()),
                ));
            }
        }
    }

    Ok(PipelineData::new(span))
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
                return Err(generic_labeled_error(
                    "Failed to parse num replicas",
                    format!("Failed to parse num replicas {}", e.to_string()),
                ));
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
                return Err(generic_labeled_error(
                    "Failed to parse durability level",
                    format!("Failed to parse durability level {}", e.to_string()),
                ));
            }
        });
    }
    if let Some(e) = expiry {
        settings.set_max_expiry(Duration::from_secs(e));
    }

    Ok(())
}
