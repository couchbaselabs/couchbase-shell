use crate::cli::buckets_builder::{
    BucketSettingsBuilder, BucketType, DurabilityLevel, JSONCloudBucketSettings,
};
use crate::cli::util::{
    cant_run_against_hosted_capella_error, cluster_identifiers_from, cluster_not_found_error,
    generic_unspanned_error, map_serde_serialize_error_to_shell_error,
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
            .category(Category::Custom("couchbase".into()))
    }

    fn usage(&self) -> &str {
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
    let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();

    let name: String = call.req(engine_state, stack, 0)?;
    let ram: i64 = call.req(engine_state, stack, 1)?;

    let bucket_type: Option<String> = call.get_flag(engine_state, stack, "type")?;
    let replicas: Option<i64> = call.get_flag(engine_state, stack, "replicas")?;
    let flush = call
        .get_flag(engine_state, stack, "flush")?
        .unwrap_or(false);
    let durability: Option<String> = call.get_flag(engine_state, stack, "durability")?;
    let expiry: Option<i64> = call.get_flag(engine_state, stack, "expiry")?;

    debug!("Running buckets create for bucket {}", &name);

    let cluster_identifiers = cluster_identifiers_from(&engine_state, stack, &state, &call, true)?;
    let guard = state.lock().unwrap();

    let mut builder = BucketSettingsBuilder::new(name).ram_quota_mb(ram as u64);
    if let Some(ref t) = bucket_type {
        builder = builder.bucket_type(match BucketType::try_from(t.as_str()) {
            Ok(bt) => bt,
            Err(_e) => {
                return Err(generic_unspanned_error("Failed to parse bucket type", format!("Failed to parse bucket type {}, allow values are couchbase, membase, memcached, ephemeral", t )));
            }
        });
    }
    if let Some(r) = replicas {
        builder = builder.num_replicas(match u32::try_from(r) {
            Ok(bt) => bt,
            Err(e) => {
                return Err(generic_unspanned_error(
                    "Failed to parse num replicas",
                    format!("Failed to parse num replicas {}", e),
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
                return Err(generic_unspanned_error("Failed to parse durability level",
                                                   format!("Failed to parse durability level {}, allow values are one, majority, majorityAndPersistActive, persistToMajority", d )));
            }
        });
    }
    if let Some(e) = expiry {
        builder = builder.max_expiry(Duration::from_secs(e as u64));
    }

    let settings = builder.build();

    for identifier in cluster_identifiers {
        let active_cluster = match guard.clusters().get(&identifier) {
            Some(c) => c,
            None => {
                return Err(cluster_not_found_error(identifier, call.span()));
            }
        };

        if active_cluster.capella_org().is_some()
            && (bucket_type.clone().is_some()
                || flush
                || durability.clone().is_some()
                || expiry.is_some())
        {
            return Err(generic_unspanned_error(
                "Capella flag cannot be used with type, flush, durability, or expiry",
                "Capella flag cannot be used with type, flush, durability, or expiry",
            ));
        }

        let response: HttpResponse;
        if let Some(plane) = active_cluster.capella_org() {
            let cloud = guard.capella_org_for_cluster(plane)?.client();
            let deadline = Instant::now().add(active_cluster.timeouts().management_timeout());
            let cluster =
                cloud.find_cluster(identifier.clone(), deadline.clone(), ctrl_c.clone())?;

            if cluster.environment() == CapellaEnvironment::Hosted {
                return Err(cant_run_against_hosted_capella_error());
            }

            let json_settings = JSONCloudBucketSettings::try_from(&settings)?;
            response = cloud.capella_request(
                CapellaRequest::CreateBucket {
                    cluster_id: cluster.id(),
                    payload: serde_json::to_string(&json_settings)
                        .map_err(map_serde_serialize_error_to_shell_error)?,
                },
                deadline,
                ctrl_c.clone(),
            )?;
        } else {
            let cluster = active_cluster.cluster();

            let form = settings.as_form(false)?;
            let payload = serde_urlencoded::to_string(&form).unwrap();

            response = cluster.http_client().management_request(
                ManagementRequest::CreateBucket { payload },
                Instant::now().add(active_cluster.timeouts().management_timeout()),
                ctrl_c.clone(),
            )?;
        }

        match response.status() {
            200 => {}
            201 => {}
            202 => {}
            _ => {
                return Err(generic_unspanned_error(
                    "Failed to create bucket",
                    format!("Failed to create bucket: {}", response.content()),
                ));
            }
        }
    }

    Ok(PipelineData::new(span))
}
