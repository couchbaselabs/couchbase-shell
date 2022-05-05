//! The `buckets get` command fetches buckets from the server.

use crate::state::State;

use crate::cli::util::{
    cluster_identifiers_from, cluster_not_found_error, generic_unspanned_error,
    validate_is_not_cloud,
};
use crate::client::ManagementRequest;
use log::debug;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

use nu_engine::CallExt;
use nu_protocol::ast::Call;
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
            .category(Category::Custom("couchbase".into()))
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
    let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();

    let cluster_identifiers = cluster_identifiers_from(&engine_state, stack, &state, &call, true)?;
    let name: String = call.req(engine_state, stack, 0)?;
    let bucket: String = call
        .get_flag(engine_state, stack, "bucket")?
        .unwrap_or_else(|| "".into());

    debug!("Running buckets flush for bucket {:?}", &bucket);

    for identifier in cluster_identifiers {
        let guard = state.lock().unwrap();
        let cluster = match guard.clusters().get(&identifier) {
            Some(c) => c,
            None => {
                return Err(cluster_not_found_error(identifier, call.span()));
            }
        };
        validate_is_not_cloud(
            cluster,
            "buckets flush cannot be run against Capella clusters",
        )?;

        let result = cluster.cluster().http_client().management_request(
            ManagementRequest::FlushBucket { name: name.clone() },
            Instant::now().add(cluster.timeouts().management_timeout()),
            ctrl_c.clone(),
        )?;

        match result.status() {
            200 => {}
            _ => {
                return Err(generic_unspanned_error(
                    "Failed to flush bucket",
                    format!("Failed to flush bucket {}", result.content()),
                ));
            }
        }
    }

    Ok(PipelineData::new(span))
}
