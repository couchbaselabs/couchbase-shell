//! The `buckets get` command fetches buckets from the server.
use crate::state::{CapellaEnvironment, State};

use crate::cli::cloud_json::JSONCloudDeleteBucketRequest;
use crate::cli::error::{
    cant_run_against_hosted_capella_error, serialize_error, unexpected_status_code_error,
};
use crate::cli::util::{cluster_identifiers_from, get_active_cluster};
use crate::client::{CapellaRequest, ManagementRequest};
use log::debug;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape};
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

#[derive(Clone)]
pub struct BucketsDrop {
    state: Arc<Mutex<State>>,
}

impl BucketsDrop {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for BucketsDrop {
    fn name(&self) -> &str {
        "buckets drop"
    }

    fn signature(&self) -> Signature {
        Signature::build("buckets drop")
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
        "Drops buckets through the HTTP API"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        buckets_drop(self.state.clone(), engine_state, stack, call, input)
    }
}

fn buckets_drop(
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
    let guard = state.lock().unwrap();

    debug!("Running buckets drop for bucket {:?}", &name);

    for identifier in cluster_identifiers {
        let cluster = get_active_cluster(identifier.clone(), &guard, span.clone())?;

        let result = if let Some(plane) = cluster.capella_org() {
            let cloud = guard.capella_org_for_cluster(plane)?.client();
            let deadline = Instant::now().add(cluster.timeouts().management_timeout());
            let cluster =
                cloud.find_cluster(identifier.clone(), deadline.clone(), ctrl_c.clone())?;

            if cluster.environment() == CapellaEnvironment::Hosted {
                return Err(cant_run_against_hosted_capella_error("buckets drop", span));
            }

            let req = JSONCloudDeleteBucketRequest::new(name.clone());
            let payload =
                serde_json::to_string(&req).map_err(|e| serialize_error(e.to_string(), span))?;
            cloud.capella_request(
                CapellaRequest::DeleteBucket {
                    cluster_id: cluster.id(),
                    payload,
                },
                deadline,
                ctrl_c.clone(),
            )?
        } else {
            cluster.cluster().http_client().management_request(
                ManagementRequest::DropBucket { name: name.clone() },
                Instant::now().add(cluster.timeouts().management_timeout()),
                ctrl_c.clone(),
            )?
        };

        match result.status() {
            200 => {}
            202 => {}
            _ => {
                return Err(unexpected_status_code_error(
                    result.status(),
                    result.content(),
                    span,
                ));
            }
        }
    }

    Ok(PipelineData::new(span))
}
