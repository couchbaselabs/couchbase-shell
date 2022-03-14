use crate::cli::util::{
    cluster_identifiers_from, cluster_not_found_error, generic_labeled_error,
    map_serde_deserialize_error_to_shell_error, validate_is_not_cloud, NuValueMap,
};
use crate::client::ManagementRequest;
use crate::state::State;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct BucketsSample {
    state: Arc<Mutex<State>>,
}

impl BucketsSample {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for BucketsSample {
    fn name(&self) -> &str {
        "buckets load-sample"
    }

    fn signature(&self) -> Signature {
        Signature::build("buckets load-sample")
            .required(
                "name",
                SyntaxShape::String,
                "the name of the bucket to load",
            )
            .named(
                "clusters",
                SyntaxShape::String,
                "the clusters which should be contacted",
                None,
            )
            .category(Category::Custom("couchbase".into()))
    }

    fn usage(&self) -> &str {
        "Load a sample bucket"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        load_sample_bucket(self.state.clone(), engine_state, stack, call, input)
    }
}

fn load_sample_bucket(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();

    let cluster_identifiers = cluster_identifiers_from(&engine_state, stack, &state, &call, true)?;
    let bucket_name: String = call.req(engine_state, stack, 0)?;

    let mut results: Vec<Value> = vec![];
    for identifier in cluster_identifiers {
        let guard = state.lock().unwrap();
        let cluster = match guard.clusters().get(&identifier) {
            Some(c) => c,
            None => {
                return Err(cluster_not_found_error(identifier));
            }
        };

        validate_is_not_cloud(
            cluster,
            "buckets sample cannot be run against cloud clusters",
        )?;

        let response = cluster.cluster().http_client().management_request(
            ManagementRequest::LoadSampleBucket {
                name: format!("[\"{}\"]", bucket_name),
            },
            Instant::now().add(cluster.timeouts().management_timeout()),
            ctrl_c.clone(),
        )?;

        match response.status() {
            202 => {}
            _ => {
                return Err(generic_labeled_error(
                    "Failed to load sample bucket",
                    format!(
                        "Failed to load sample bucket {}",
                        response.content().to_string()
                    ),
                ))
            }
        }

        let resp: Vec<String> = serde_json::from_str(response.content())
            .map_err(map_serde_deserialize_error_to_shell_error)?;
        for r in resp {
            let mut collected = NuValueMap::default();
            collected.add_string("cluster", identifier.clone(), span);
            collected.add_string("results", r, span);
            results.push(collected.into_value(span));
        }
    }

    Ok(Value::List {
        vals: results,
        span: call.head,
    }
    .into_pipeline_data())
}
