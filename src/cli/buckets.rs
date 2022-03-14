use crate::cli::buckets_builder::{BucketSettings, JSONBucketSettings, JSONCloudBucketSettings};
use crate::cli::buckets_get::bucket_to_nu_value;
use crate::cli::util::{
    cant_run_against_hosted_capella_error, cluster_identifiers_from, cluster_not_found_error,
    generic_labeled_error, map_serde_deserialize_error_to_shell_error,
};
use crate::client::{CapellaRequest, ManagementRequest};
use crate::state::{CapellaEnvironment, State};
use log::debug;
use std::convert::TryFrom;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

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
            .category(Category::Custom("couchbase".into()))
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

    let cluster_identifiers = cluster_identifiers_from(&engine_state, stack, &state, &call, true)?;

    debug!("Running buckets");

    let guard = state.lock().unwrap();
    let mut results = vec![];
    for identifier in cluster_identifiers {
        let cluster = match guard.clusters().get(&identifier) {
            Some(c) => c,
            None => {
                return Err(cluster_not_found_error(identifier));
            }
        };

        if let Some(plane) = cluster.capella_org() {
            let cloud = guard.capella_org_for_cluster(plane)?.client();
            let deadline = Instant::now().add(cluster.timeouts().management_timeout());
            let cluster = cloud.find_cluster(identifier.clone(), deadline, ctrl_c.clone())?;

            if cluster.environment() == CapellaEnvironment::Hosted {
                return Err(cant_run_against_hosted_capella_error());
            }

            let response = cloud.capella_request(
                CapellaRequest::GetBuckets {
                    cluster_id: cluster.id(),
                },
                deadline,
                ctrl_c.clone(),
            )?;
            if response.status() != 200 {
                return Err(generic_labeled_error(
                    "Failed to get buckets from Capella",
                    format!(
                        "Failed to get buckets returned {}, content: {}",
                        response.status(),
                        response.content()
                    ),
                ));
            }

            let content: Vec<JSONCloudBucketSettings> = serde_json::from_str(response.content())
                .map_err(map_serde_deserialize_error_to_shell_error)?;
            for bucket in content.into_iter() {
                results.push(bucket_to_nu_value(
                    BucketSettings::try_from(bucket)?,
                    identifier.clone(),
                    true,
                    span,
                ));
            }
        } else {
            let response = cluster.cluster().http_client().management_request(
                ManagementRequest::GetBuckets,
                Instant::now().add(cluster.timeouts().management_timeout()),
                ctrl_c.clone(),
            )?;

            let content: Vec<JSONBucketSettings> = serde_json::from_str(response.content())
                .map_err(map_serde_deserialize_error_to_shell_error)?;

            for bucket in content.into_iter() {
                results.push(bucket_to_nu_value(
                    BucketSettings::try_from(bucket)?,
                    identifier.clone(),
                    false,
                    span,
                ));
            }
        }
    }

    Ok(Value::List {
        vals: results,
        span: call.head,
    }
    .into_pipeline_data())
}
