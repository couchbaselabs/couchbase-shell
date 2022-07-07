use crate::cli::buckets_builder::{BucketSettings, JSONBucketSettings, JSONCloudBucketSettings};
use crate::cli::buckets_get::bucket_to_nu_value;
use crate::cli::util::{cluster_identifiers_from, get_active_cluster};
use crate::client::{CapellaRequest, ManagementRequest};
use crate::state::{CapellaEnvironment, State};
use log::debug;
use std::convert::TryFrom;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

use crate::cli::error::{
    cant_run_against_hosted_capella_error, deserialize_error, generic_error,
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
    let guard = state.lock().unwrap();

    debug!("Running buckets");

    let mut results = vec![];
    for identifier in cluster_identifiers {
        let cluster = get_active_cluster(identifier.clone(), &guard, span.clone())?;

        if let Some(plane) = cluster.capella_org() {
            let cloud = guard.capella_org_for_cluster(plane)?.client();
            let deadline = Instant::now().add(cluster.timeouts().management_timeout());
            let cluster = cloud.find_cluster(identifier.clone(), deadline, ctrl_c.clone())?;

            if cluster.environment() == CapellaEnvironment::Hosted {
                return Err(cant_run_against_hosted_capella_error("buckets", span));
            }

            let response = cloud.capella_request(
                CapellaRequest::GetBuckets {
                    cluster_id: cluster.id(),
                },
                deadline,
                ctrl_c.clone(),
            )?;
            if response.status() != 200 {
                return Err(unexpected_status_code_error(
                    response.status(),
                    response.content(),
                    span,
                ));
            }

            let content: Vec<JSONCloudBucketSettings> = serde_json::from_str(response.content())
                .map_err(|e| deserialize_error(e.to_string(), span))?;
            for bucket in content.into_iter() {
                results.push(bucket_to_nu_value(
                    BucketSettings::try_from(bucket).map_err(|e| {
                        generic_error(format!("Invalid setting {}", e.to_string()), None, span)
                    })?,
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
                        generic_error(format!("Invalid setting {}", e.to_string()), None, span)
                    })?,
                    identifier.clone(),
                    false,
                    span,
                ));
            }
        }
    }

    Ok(Value::List {
        vals: results,
        span,
    }
    .into_pipeline_data())
}
