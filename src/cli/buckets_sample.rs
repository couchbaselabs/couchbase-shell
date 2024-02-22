use crate::cli::error::{
    client_error_to_shell_error, deserialize_error, unexpected_status_code_error,
};
use crate::cli::util::{
    cluster_identifiers_from, get_active_cluster, validate_is_not_cloud, NuValueMap,
};
use crate::client::ManagementRequest;
use crate::state::State;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};
use serde::Deserialize;
use serde_derive::Serialize;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

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
                "databases",
                SyntaxShape::String,
                "the databases which should be contacted",
                None,
            )
            .category(Category::Custom("couchbase".to_string()))
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

#[derive(Debug, Deserialize, Serialize)]
struct SampleLoadTask {
    #[serde(rename = "taskId", default)]
    task_id: String,
    sample: String,
    bucket: String,
}

#[derive(Debug, Deserialize)]
struct SampleLoadTasks {
    tasks: Vec<SampleLoadTask>,
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

    let cluster_identifiers = cluster_identifiers_from(engine_state, stack, &state, call, true)?;
    let guard = state.lock().unwrap();
    let bucket_name: String = call.req(engine_state, stack, 0)?;

    let mut results: Vec<Value> = vec![];
    for identifier in cluster_identifiers {
        let cluster = get_active_cluster(identifier.clone(), &guard, span)?;

        validate_is_not_cloud(cluster, "buckets sample", span)?;

        let response = cluster
            .cluster()
            .http_client()
            .management_request(
                ManagementRequest::LoadSampleBucket {
                    name: format!("[\"{}\"]", bucket_name),
                },
                Instant::now().add(cluster.timeouts().management_timeout()),
                ctrl_c.clone(),
            )
            .map_err(|e| client_error_to_shell_error(e, span))?;

        match response.status() {
            202 => {}
            _ => {
                return Err(unexpected_status_code_error(
                    response.status(),
                    response.content(),
                    span,
                ))
            }
        }

        let resp: SampleLoadTasks = serde_json::from_str(response.content())
            .map_err(|e| deserialize_error(e.to_string(), span))?;
        for r in resp.tasks {
            let mut collected = NuValueMap::default();
            collected.add_string("cluster", identifier.clone(), span);
            collected.add_string("results", serde_json::to_string(&r).unwrap(), span);
            results.push(collected.into_value(span));
        }
    }

    Ok(Value::List {
        vals: results,
        span,
    }
    .into_pipeline_data())
}
