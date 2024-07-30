use crate::client::cloud_json::{ClusterCreateRequest, Provider};
use crate::state::State;
use log::{debug, info};
use std::convert::TryFrom;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

use crate::cli::error::{client_error_to_shell_error, serialize_error};
use crate::cli::util::{find_org_id, find_project_id};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape, Value};
use uuid::Uuid;

#[derive(Clone)]
pub struct ClustersCreate {
    state: Arc<Mutex<State>>,
}

impl ClustersCreate {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for ClustersCreate {
    fn name(&self) -> &str {
        "clusters create"
    }

    fn signature(&self) -> Signature {
        Signature::build("clusters create")
            .named("name", SyntaxShape::String, "the name of the cluster", None)
            .named("provider", SyntaxShape::String, "the cloud provider", None)
            .named(
                "version",
                SyntaxShape::String,
                "the couchbase server version",
                None,
            )
            .named(
                "capella",
                SyntaxShape::String,
                "the Capella organization to use",
                None,
            )
            .named(
                "nodes",
                SyntaxShape::Int,
                "the number of nodes in the cluster",
                None,
            )
            .category(Category::Custom("couchbase".to_string()))
    }

    fn usage(&self) -> &str {
        "Creates a new cluster on the active Capella organization"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        clusters_create(self.state.clone(), engine_state, stack, call, input)
    }
}

fn clusters_create(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();

    let definition = match input.into_value(span)? {
        Value::Nothing { .. } => {
            let provider = match call.get_flag::<String>(engine_state, stack, "provider")? {
                Some(p) => Provider::try_from(p.as_str())?,
                None => {
                    return Err(ShellError::GenericError {
                        error: "no provider specified".to_string(),
                        msg: "".to_string(),
                        span: None,
                        help: Some(
                            "Please specify a cloud provider using the '--provider' flag".into(),
                        ),
                        inner: vec![],
                    })
                }
            };
            let name = call
                .get_flag(engine_state, stack, "name")?
                .unwrap_or_else(|| {
                    info!("Cluster name not specified, a randomly generated name will be used");
                    random_cluster_name()
                });
            let nodes = call
                .get_flag(engine_state, stack, "nodes")?
                .unwrap_or_else(|| {
                    info!("Number of nodes not specified, defaulting to 1");
                    1
                });

            let version = call.get_flag(engine_state, stack, "version")?;
            ClusterCreateRequest::new(name, provider, version, nodes)
        }
        Value::String { val, .. } => {
            serde_json::from_str(val.as_str()).map_err(|e| serialize_error(e.to_string(), span))?
        }
        _ => {
            return Err(ShellError::GenericError {
                error: "cluster definition must be a string".to_string(),
                msg: "".to_string(),
                span: None,
                help: None,
                inner: vec![],
            })
        }
    };

    let capella = call.get_flag(engine_state, stack, "capella")?;

    debug!("Running clusters create for {:?}", definition);

    let guard = state.lock().unwrap();
    let control = if let Some(c) = capella {
        guard.get_capella_org(c)
    } else {
        guard.active_capella_org()
    }?;
    let client = control.client();
    let deadline = Instant::now().add(control.timeout());

    let org_id = find_org_id(ctrl_c.clone(), &client, deadline, span)?;
    let project_id = find_project_id(
        ctrl_c.clone(),
        guard.active_project()?,
        &client,
        deadline,
        span,
        org_id.clone(),
    )?;

    let payload =
        serde_json::to_string(&definition).map_err(|e| serialize_error(e.to_string(), span))?;
    client
        .create_cluster(org_id, project_id, payload, deadline, ctrl_c)
        .map_err(|e| client_error_to_shell_error(e, span))?;

    Ok(PipelineData::empty())
}

fn random_cluster_name() -> String {
    let mut uuid = Uuid::new_v4().to_string();
    uuid.truncate(6);
    format!("cbshell-cluster-{}", uuid)
}
