//! The `buckets get` command fetches buckets from the server.

use crate::state::State;

use crate::cli::util::{cluster_identifiers_from, get_active_cluster, validate_is_not_cloud};
use crate::client::ManagementRequest;
use log::debug;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

use crate::cli::error::{
    bucket_not_found_error, client_error_to_shell_error, unexpected_status_code_error,
};
use nu_engine::command_prelude::Call;
use nu_engine::CallExt;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape};

#[derive(Clone)]
pub struct BucketsFlush {
    state: Arc<Mutex<State>>,
}

impl BucketsFlush {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for BucketsFlush {
    fn name(&self) -> &str {
        "buckets flush"
    }

    fn signature(&self) -> Signature {
        Signature::build("buckets flush")
            .required("name", SyntaxShape::String, "the name of the bucket")
            .named(
                "clusters",
                SyntaxShape::String,
                "the clusters which should be contacted",
                None,
            )
            .category(Category::Custom("couchbase".to_string()))
    }

    fn usage(&self) -> &str {
        "Flushes buckets through the HTTP API"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        buckets_flush(self.state.clone(), engine_state, stack, call, input)
    }
}

fn buckets_flush(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let signals = engine_state.signals().clone();

    let cluster_identifiers = cluster_identifiers_from(engine_state, stack, &state, call, true)?;
    let name: String = call.req(engine_state, stack, 0)?;
    let bucket: String = call
        .get_flag(engine_state, stack, "bucket")?
        .unwrap_or_default();

    debug!("Running buckets flush for bucket {:?}", &bucket);

    for identifier in cluster_identifiers {
        let guard = state.lock().unwrap();
        let cluster = get_active_cluster(identifier.clone(), &guard, span)?;
        validate_is_not_cloud(cluster, "buckets flush", span)?;

        let result = cluster
            .cluster()
            .http_client()
            .management_request(
                ManagementRequest::FlushBucket { name: name.clone() },
                Instant::now().add(cluster.timeouts().management_timeout()),
                signals.clone(),
            )
            .map_err(|e| client_error_to_shell_error(e, span))?;

        match result.status() {
            200 => {}
            404 => {
                if result
                    .content()?
                    .to_string()
                    .to_lowercase()
                    .contains("resource not found")
                {
                    return Err(bucket_not_found_error(name, span));
                }
            }
            _ => {
                return Err(unexpected_status_code_error(
                    result.status(),
                    result.content()?,
                    span,
                ));
            }
        }
    }

    Ok(PipelineData::empty())
}
