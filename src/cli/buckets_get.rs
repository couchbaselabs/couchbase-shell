//! The `buckets get` command fetches buckets from the server.
use crate::state::State;

use crate::cli::buckets_builder::{BucketSettings, JSONBucketSettings};
use crate::cli::util::{cluster_identifiers_from, get_active_cluster, NuValueMap};
use crate::client::ManagementRequest;
use log::debug;
use std::convert::TryFrom;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

use crate::cli::error::{
    bucket_not_found_error, client_error_to_shell_error, deserialize_error,
    malformed_response_error, unexpected_status_code_error,
};
use crate::remote_cluster::RemoteCluster;
use nu_engine::command_prelude::Call;
use nu_engine::CallExt;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signals, Signature, Span, SyntaxShape,
    Value,
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
            .category(Category::Custom("couchbase".to_string()))
    }

    fn description(&self) -> &str {
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
    let signals = engine_state.signals().clone();

    let cluster_identifiers = cluster_identifiers_from(engine_state, stack, &state, call, true)?;
    let bucket: String = call.req(engine_state, stack, 0)?;

    debug!("Running buckets get for bucket {:?}", &bucket);

    let mut results: Vec<Value> = vec![];
    for identifier in cluster_identifiers {
        let guard = state.lock().unwrap();
        let active_cluster = get_active_cluster(identifier.clone(), &guard, span)?;

        let content = get_server_bucket(active_cluster, bucket.clone(), signals.clone(), span)?;

        results.push(bucket_to_nu_value(
            content,
            identifier,
            active_cluster.is_capella(),
            span,
        ));
    }

    Ok(Value::List {
        vals: results,
        internal_span: call.head,
    }
    .into_pipeline_data())
}

pub fn get_server_bucket(
    cluster: &RemoteCluster,
    bucket: String,
    signals: Signals,
    span: Span,
) -> Result<BucketSettings, ShellError> {
    let response = cluster
        .cluster()
        .http_client()
        .management_request(
            ManagementRequest::GetBucket {
                name: bucket.clone(),
            },
            Instant::now().add(cluster.timeouts().query_timeout()),
            signals.clone(),
        )
        .map_err(|e| client_error_to_shell_error(e, span))?;

    let status = response.status();
    let content = response.content()?;
    check_response(status, content.clone(), bucket.clone(), span)?;

    let json: JSONBucketSettings =
        serde_json::from_str(&content).map_err(|e| deserialize_error(e.to_string(), span))?;

    BucketSettings::try_from(json).map_err(|e| {
        malformed_response_error(
            "Could not parse bucket settings",
            format!("Error: {}, response content: {:?}", e, content),
            span,
        )
    })
}

pub(crate) fn check_response(
    status: u16,
    content: String,
    bucket: String,
    span: Span,
) -> Result<(), ShellError> {
    match status {
        200 => {}
        404 => {
            if content.to_lowercase().contains("resource not found") {
                return Err(bucket_not_found_error(bucket, span));
            }
        }
        _ => {
            return Err(unexpected_status_code_error(status, content, span));
        }
    };
    Ok(())
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
    collected.add_i64("replicas", bucket.num_replicas().unwrap() as i64, span);
    collected.add_string(
        "min_durability_level",
        bucket.minimum_durability_level().to_string(),
        span,
    );
    collected.add(
        "ram_quota",
        Value::Filesize {
            val: (bucket.ram_quota_mb() * 1024 * 1024) as i64,
            internal_span: span,
        },
    );
    collected.add_bool("flush_enabled", bucket.flush_enabled(), span);
    collected.add_bool("cloud", is_cloud, span);
    collected.add_i64("max_expiry", bucket.max_expiry(), span);
    collected.into_value(span)
}
