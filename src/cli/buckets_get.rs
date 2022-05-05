//! The `buckets get` command fetches buckets from the server.

use crate::state::{CapellaEnvironment, State};

use crate::cli::buckets_builder::{BucketSettings, JSONBucketSettings, JSONCloudBucketSettings};
use crate::cli::util::{
    cant_run_against_hosted_capella_error, cluster_identifiers_from, cluster_not_found_error,
    generic_unspanned_error, map_serde_deserialize_error_to_shell_error, NuValueMap,
};
use crate::client::{CapellaRequest, ManagementRequest};
use log::debug;
use std::convert::TryFrom;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct BucketsGet {
    state: Arc<Mutex<State>>,
}

impl BucketsGet {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for BucketsGet {
    fn name(&self) -> &str {
        "buckets get"
    }

    fn signature(&self) -> Signature {
        Signature::build("buckets get")
            .required("bucket", SyntaxShape::String, "the name of the bucket")
            .named(
                "clusters",
                SyntaxShape::String,
                "the clusters which should be contacted",
                None,
            )
            .category(Category::Custom("couchbase".into()))
    }

    fn usage(&self) -> &str {
        "Fetches buckets through the HTTP API"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        buckets_get(self.state.clone(), engine_state, stack, call, input)
    }
}

fn buckets_get(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();

    let cluster_identifiers = cluster_identifiers_from(&engine_state, stack, &state, &call, true)?;
    let bucket: String = call.req(engine_state, stack, 0)?;

    debug!("Running buckets get for bucket {:?}", &bucket);

    let mut results: Vec<Value> = vec![];
    for identifier in cluster_identifiers {
        let guard = state.lock().unwrap();
        let cluster = match guard.clusters().get(&identifier) {
            Some(c) => c,
            None => {
                return Err(cluster_not_found_error(identifier, call.span()));
            }
        };

        if let Some(plane) = cluster.capella_org() {
            let cloud = guard.capella_org_for_cluster(plane)?.client();
            let cloud_cluster = cloud.find_cluster(
                identifier.clone(),
                Instant::now().add(cluster.timeouts().query_timeout()),
                ctrl_c.clone(),
            )?;

            if cloud_cluster.environment() == CapellaEnvironment::Hosted {
                return Err(cant_run_against_hosted_capella_error());
            }

            let response = cloud.capella_request(
                CapellaRequest::GetBuckets {
                    cluster_id: cloud_cluster.id(),
                },
                Instant::now().add(cluster.timeouts().query_timeout()),
                ctrl_c.clone(),
            )?;
            if response.status() != 200 {
                return Err(generic_unspanned_error(
                    "Failed to get buckets",
                    format!("Failed to get buckets {}", response.content()),
                ));
            }

            let content: Vec<JSONCloudBucketSettings> = serde_json::from_str(response.content())
                .map_err(map_serde_deserialize_error_to_shell_error)?;
            let mut bucket_settings: Option<JSONCloudBucketSettings> = None;

            for b in content.into_iter() {
                if b.name() == bucket.clone() {
                    bucket_settings = Some(b);
                    break;
                }
            }

            if let Some(b) = bucket_settings {
                results.push(bucket_to_nu_value(
                    BucketSettings::try_from(b)?,
                    identifier,
                    true,
                    span,
                ));
            } else {
                return Err(generic_unspanned_error(
                    "Bucket not found",
                    format!("Bucket {} not found", bucket),
                ));
            }
        } else {
            let response = cluster.cluster().http_client().management_request(
                ManagementRequest::GetBucket {
                    name: bucket.clone(),
                },
                Instant::now().add(cluster.timeouts().query_timeout()),
                ctrl_c.clone(),
            )?;

            let content: JSONBucketSettings = serde_json::from_str(response.content())
                .map_err(map_serde_deserialize_error_to_shell_error)?;
            results.push(bucket_to_nu_value(
                BucketSettings::try_from(content)?,
                identifier,
                false,
                span,
            ));
        }
    }

    Ok(Value::List {
        vals: results,
        span: call.head,
    }
    .into_pipeline_data())
}

pub(crate) fn bucket_to_nu_value(
    bucket: BucketSettings,
    cluster_name: String,
    is_cloud: bool,
    span: Span,
) -> Value {
    let mut collected = NuValueMap::default();
    collected.add_string("cluster", cluster_name, span);
    collected.add_string("name", bucket.name(), span);
    collected.add_string("type", bucket.bucket_type().to_string(), span);
    collected.add_i64("replicas", bucket.num_replicas() as i64, span);
    collected.add_string(
        "min_durability_level",
        bucket.minimum_durability_level().to_string(),
        span,
    );
    collected.add(
        "ram_quota",
        Value::Filesize {
            val: (bucket.ram_quota_mb() * 1024 * 1024) as i64,
            span,
        },
    );
    collected.add_bool("flush_enabled", bucket.flush_enabled(), span);
    collected.add_string(
        "status",
        bucket.status().unwrap_or(&"".to_string()).clone(),
        span,
    );
    collected.add_bool("cloud", is_cloud, span);
    collected.into_value(span)
}
