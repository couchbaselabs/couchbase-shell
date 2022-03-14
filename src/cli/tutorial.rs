use crate::client::ManagementRequest;
use crate::state::State;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, IntoPipelineData, PipelineData, ShellError, Signature, Value};

#[derive(Clone)]
pub struct Tutorial {
    state: Arc<Mutex<State>>,
}

impl Tutorial {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for Tutorial {
    fn name(&self) -> &str {
        "tutorial"
    }

    fn signature(&self) -> Signature {
        Signature::build("tutorial").category(Category::Custom("couchbase".into()))
    }

    fn usage(&self) -> &str {
        "Run the Couchbase Shell tutorial"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        run_tutorial(self.state.clone(), engine_state, stack, call, input)
    }
}

fn run_tutorial(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    _stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();

    let guard = state.lock().unwrap();
    let tutorial = guard.tutorial();
    let exists = match guard.active_cluster() {
        Some(active_cluster) => {
            if active_cluster.capella_org().is_none() {
                let resp = active_cluster.cluster().http_client().management_request(
                    ManagementRequest::GetBucket {
                        name: "travel-sample".into(),
                    },
                    Instant::now().add(active_cluster.timeouts().management_timeout()),
                    ctrl_c,
                );

                match resp {
                    Ok(r) => matches!(r.status(), 200),
                    Err(_) => false,
                }
            } else {
                // Bit of a hack, if the user is on cloud then they can't enable travel-sample
                true
            }
        }
        None => true,
    };

    Ok(Value::String {
        val: tutorial.current_step(exists),
        span: call.head,
    }
    .into_pipeline_data())
}
