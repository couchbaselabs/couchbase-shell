//! The `buckets get` command fetches buckets from the server.
use crate::state::{CapellaEnvironment, State};

use crate::cli::cloud_json::JSONCloudDeleteBucketRequest;
use crate::cli::util::{
    cluster_identifiers_from, cluster_not_found_error, generic_unspanned_error,
    map_serde_serialize_error_to_shell_error,
};
use crate::client::{CapellaRequest, HttpResponse, ManagementRequest};
use log::debug;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape};

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
        let cluster = match guard.clusters().get(&identifier) {
            Some(c) => c,
            None => {
                return Err(cluster_not_found_error(identifier, call.span()));
            }
        };

        let result: HttpResponse;
        if let Some(plane) = cluster.capella_org() {
            let cloud = guard.capella_org_for_cluster(plane)?.client();
            let deadline = Instant::now().add(cluster.timeouts().management_timeout());
            let cluster =
                cloud.find_cluster(identifier.clone(), deadline.clone(), ctrl_c.clone())?;

            if cluster.environment() == CapellaEnvironment::Hosted {
                return Err(generic_unspanned_error(
                    "buckets drop cannot  be run against hosted Capella clusters",
                    "buckets drop cannot  be run against hosted Capella clusters",
                ));
            }

            let req = JSONCloudDeleteBucketRequest::new(name.clone());
            let payload =
                serde_json::to_string(&req).map_err(map_serde_serialize_error_to_shell_error)?;
            result = cloud.capella_request(
                CapellaRequest::DeleteBucket {
                    cluster_id: cluster.id(),
                    payload,
                },
                deadline,
                ctrl_c.clone(),
            )?;
        } else {
            result = cluster.cluster().http_client().management_request(
                ManagementRequest::DropBucket { name: name.clone() },
                Instant::now().add(cluster.timeouts().management_timeout()),
                ctrl_c.clone(),
            )?;
        }

        match result.status() {
            200 => {}
            202 => {}
            _ => {
                return Err(generic_unspanned_error(
                    "Failed to drop bucket",
                    format!("Failed to drop bucket: {}", result.content()),
                ));
            }
        }
    }

    Ok(PipelineData::new(span))
}
