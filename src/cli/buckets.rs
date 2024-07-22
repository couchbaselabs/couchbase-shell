use crate::cli::buckets_builder::{BucketSettings, JSONBucketSettings};
use crate::cli::buckets_get::bucket_to_nu_value;
use crate::cli::util::{cluster_identifiers_from, get_active_cluster, validate_is_not_cloud};
use crate::client::ManagementRequest;
use crate::state::State;
use log::debug;
use std::convert::TryFrom;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

use crate::cli::error::{
    client_error_to_shell_error, deserialize_error, malformed_response_error,
    unexpected_status_code_error,
};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct Buckets {
    state: Arc<Mutex<State>>,
}

impl Buckets {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for Buckets {
    fn name(&self) -> &str {
        "buckets"
    }

    fn signature(&self) -> Signature {
        Signature::build("buckets")
            .named(
                "clusters",
                SyntaxShape::String,
                "the clusters which should be contacted",
                None,
            )
            .category(Category::Custom("couchbase".to_string()))
    }

    fn usage(&self) -> &str {
        "Perform bucket management operations"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        buckets_get_all(self.state.clone(), engine_state, stack, call, input)
    }
}

fn buckets_get_all(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();

    let cluster_identifiers = cluster_identifiers_from(engine_state, stack, &state, call, true)?;
    let guard = state.lock().unwrap();

    debug!("Running buckets");

    let mut results = vec![];
    for identifier in cluster_identifiers {
        let cluster = get_active_cluster(identifier.clone(), &guard, span)?;
        validate_is_not_cloud(cluster, "buckets", span)?;

        let response = cluster
            .cluster()
            .http_client()
            .management_request(
                ManagementRequest::GetBuckets,
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

        let content: Vec<JSONBucketSettings> = serde_json::from_str(response.content())
            .map_err(|e| deserialize_error(e.to_string(), span))?;

        for bucket in content.into_iter() {
            results.push(bucket_to_nu_value(
                BucketSettings::try_from(bucket).map_err(|e| {
                    malformed_response_error(
                        "Could not parse bucket settings",
                        format!("Error: {}, response content: {}", e, response.content()),
                        span,
                    )
                })?,
                identifier.clone(),
                false,
                span,
            ));
        }
    }

    Ok(Value::List {
        vals: results,
        internal_span: span,
    }
    .into_pipeline_data())
}
