use crate::cli::error::{
    client_error_to_shell_error, deserialize_error, no_active_cluster_error,
    unexpected_status_code_error,
};
use crate::cli::util::convert_json_value_to_nu_value;
use crate::client::ManagementRequest;
use crate::state::State;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape};
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

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
            .category(Category::Custom("couchbase".to_string()))
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

    let name: String = call.req(engine_state, stack, 0)?;

    let guard = state.lock().unwrap();
    let active_cluster = match guard.active_cluster() {
        Some(c) => c,
        None => {
            return Err(no_active_cluster_error(span));
        }
    };

    let response = active_cluster
        .cluster()
        .http_client()
        .management_request(
            ManagementRequest::GetBucket { name },
            Instant::now().add(active_cluster.timeouts().management_timeout()),
            ctrl_c,
        )
        .map_err(|e| client_error_to_shell_error(e, span))?;

    match response.status() {
        200 => {}
        _ => {
            return Err(unexpected_status_code_error(
                response.status(),
                response.content()?,
                span,
            ));
        }
    }

    let content = serde_json::from_str(&response.content()?)
        .map_err(|e| deserialize_error(e.to_string(), span))?;
    let converted = convert_json_value_to_nu_value(&content, span)?;

    Ok(converted.into_pipeline_data())
}
