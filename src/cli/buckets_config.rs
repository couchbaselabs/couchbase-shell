use crate::cli::util::{
    convert_json_value_to_nu_value, generic_unspanned_error,
    map_serde_deserialize_error_to_shell_error, no_active_cluster_error, validate_is_not_cloud,
};
use crate::client::ManagementRequest;
use crate::state::State;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape};

#[derive(Clone)]
pub struct BucketsConfig {
    state: Arc<Mutex<State>>,
}

impl BucketsConfig {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for BucketsConfig {
    fn name(&self) -> &str {
        "buckets config"
    }

    fn signature(&self) -> Signature {
        Signature::build("buckets config")
            .required("name", SyntaxShape::String, "the name of the bucket")
            .category(Category::Custom("couchbase".into()))
    }

    fn usage(&self) -> &str {
        "Shows the bucket config (low level)"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        buckets(self.state.clone(), engine_state, stack, call, input)
    }
}

fn buckets(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();

    let bucket_name = call.req(engine_state, stack, 0)?;

    let guard = state.lock().unwrap();
    let active_cluster = match guard.active_cluster() {
        Some(c) => c,
        None => {
            return Err(no_active_cluster_error());
        }
    };
    let cluster = active_cluster.cluster();

    validate_is_not_cloud(
        active_cluster,
        "buckets config cannot be run against Capella clusters",
    )?;

    let response = cluster.http_client().management_request(
        ManagementRequest::GetBucket { name: bucket_name },
        Instant::now().add(active_cluster.timeouts().management_timeout()),
        ctrl_c,
    )?;

    match response.status() {
        200 => {}
        _ => {
            return Err(generic_unspanned_error(
                "Failed to get bucket config",
                format!(
                    "Failed to get bucket config {}",
                    response.content().to_string()
                ),
            ))
        }
    }

    let content = serde_json::from_str(response.content())
        .map_err(map_serde_deserialize_error_to_shell_error)?;
    let converted = convert_json_value_to_nu_value(&content, span)?;

    Ok(converted.into_pipeline_data())
}
