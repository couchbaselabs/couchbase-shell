use crate::cli::cloud_json::{JSONCloudCluster, JSONCloudClusterV3};
use crate::client::CapellaRequest;
use crate::state::{CapellaEnvironment, State};
use log::debug;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

use crate::cli::util::{
    generic_unspanned_error, map_serde_deserialize_error_to_shell_error, NuValueMap,
};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape};

#[derive(Clone)]
pub struct ClustersGet {
    state: Arc<Mutex<State>>,
}

impl ClustersGet {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for ClustersGet {
    fn name(&self) -> &str {
        "clusters get"
    }

    fn signature(&self) -> Signature {
        Signature::build("clusters get")
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
        "Gets a cluster from the active Capella organization"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        clusters_get(self.state.clone(), engine_state, stack, call, input)
    }
}

fn clusters_get(
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

    debug!("Running clusters get for {}", &name);

    let guard = state.lock().unwrap();
    let control = if let Some(c) = capella {
        guard.capella_org_for_cluster(c)
    } else {
        guard.active_capella_org()
    }?;
    let client = control.client();

    let deadline = Instant::now().add(control.timeout());
    let cluster = client.find_cluster(name, deadline, ctrl_c.clone())?;
    if cluster.environment() == CapellaEnvironment::Hosted {
        let response = client.capella_request(
            CapellaRequest::GetClusterV3 {
                cluster_id: cluster.id(),
            },
            deadline,
            ctrl_c,
        )?;
        if response.status() != 200 {
            return Err(generic_unspanned_error(
                "Failed to get clusters",
                format!("Failed to get clusters {}", response.content()),
            ));
        };
        let cluster: JSONCloudClusterV3 = serde_json::from_str(response.content())
            .map_err(map_serde_deserialize_error_to_shell_error)?;

        let mut collected = NuValueMap::default();
        collected.add_string("name", cluster.name(), span);
        collected.add_string("id", cluster.id(), span);
        collected.add_string("status", cluster.status(), span);
        collected.add_string(
            "endpoint_srv",
            cluster.endpoints_srv().unwrap_or_else(|| "".to_string()),
            span,
        );
        collected.add_string("version", cluster.version_name(), span);
        collected.add_string("tenant_id", cluster.tenant_id(), span);
        collected.add_string("project_id", cluster.project_id(), span);
        collected.add_string("environment", cluster.environment(), span);

        return Ok(collected.into_pipeline_data(span));
    }

    let response = client.capella_request(
        CapellaRequest::GetCluster {
            cluster_id: cluster.id(),
        },
        deadline,
        ctrl_c,
    )?;
    if response.status() != 200 {
        return Err(generic_unspanned_error(
            "Failed to get cluster",
            format!("Failed to get cluster {}", response.content()),
        ));
    };
    let cluster: JSONCloudCluster = serde_json::from_str(response.content())
        .map_err(map_serde_deserialize_error_to_shell_error)?;

    let mut collected = NuValueMap::default();
    collected.add_string("name", cluster.name(), span);
    collected.add_string("id", cluster.id(), span);
    collected.add_string("status", cluster.status(), span);
    collected.add_string("endpoint_urls", cluster.endpoints_url().join(","), span);
    collected.add_string(
        "endpoint_srv",
        cluster.endpoints_srv().unwrap_or_else(|| "".to_string()),
        span,
    );
    collected.add_string("version", cluster.version_name(), span);
    collected.add_string("cloud_id", cluster.cloud_id(), span);
    collected.add_string("tenant_id", cluster.tenant_id(), span);
    collected.add_string("project_id", cluster.project_id(), span);

    return Ok(collected.into_pipeline_data(span));
}
