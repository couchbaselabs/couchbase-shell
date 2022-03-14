use crate::cli::util::generic_labeled_error;
use crate::client::CapellaRequest;
use crate::state::{CapellaEnvironment, State};
use log::debug;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape};

#[derive(Clone)]
pub struct ClustersDrop {
    state: Arc<Mutex<State>>,
}

impl ClustersDrop {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for ClustersDrop {
    fn name(&self) -> &str {
        "clusters drop"
    }

    fn signature(&self) -> Signature {
        Signature::build("clusters drop")
            .required("name", SyntaxShape::String, "the name of the cluster")
            .named(
                "capella",
                SyntaxShape::String,
                "the Capella organization to use",
                None,
            )
            .category(Category::Custom("couchbase".into()))
    }

    fn usage(&self) -> &str {
        "Deletes a cluster from the active Capella organization"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        clusters_drop(self.state.clone(), engine_state, stack, call, input)
    }
}

fn clusters_drop(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();

    let name: String = call.req(engine_state, stack, 0)?;
    let capella = call.get_flag(engine_state, stack, "capella")?;

    debug!("Running clusters drop for {}", &name);

    let guard = state.lock().unwrap();
    let control = if let Some(c) = capella.clone() {
        guard.capella_org_for_cluster(c)
    } else {
        guard.active_capella_org()
    }?;

    let client = control.client();

    let deadline = Instant::now().add(control.timeout());
    let cluster = client.find_cluster(name, deadline.clone(), ctrl_c.clone())?;
    let response = if cluster.environment() == CapellaEnvironment::Hosted {
        client.capella_request(
            CapellaRequest::DeleteClusterV3 {
                cluster_id: cluster.id(),
            },
            deadline,
            ctrl_c,
        )
    } else {
        client.capella_request(
            CapellaRequest::DeleteCluster {
                cluster_id: cluster.id(),
            },
            deadline,
            ctrl_c,
        )
    }?;
    if response.status() != 202 {
        return Err(generic_labeled_error(
            "Failed to drop cluster",
            format!("Failed to drop cluster {}", response.content()),
        ));
    };

    return Ok(PipelineData::new(span));
}
