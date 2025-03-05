use crate::client::cloud_json::{ClusterCreateRequest, Provider};
use crate::state::State;
use log::{debug, info};
use std::convert::TryFrom;
use std::sync::{Arc, Mutex};

use crate::cli::error::{client_error_to_shell_error, serialize_error};
use crate::cli::generic_error;
use crate::cli::util::{find_org_id, find_project_id};
use nu_engine::command_prelude::Call;
use nu_engine::CallExt;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Value};
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
            .named(
                "project",
                SyntaxShape::String,
                "the Capella project to use",
                None,
            )
            .category(Category::Custom("couchbase".to_string()))
    }

    fn description(&self) -> &str {
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

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Create a cluster from a saved definition",
                example: "cat gcp-cluster-def.json | clusters create",
                result: None,
            },
            Example {
                description: "Create a 3 node cluster with AWS",
                example: "clusters create --provider aws --nodes 3 --name testing",
                result: None,
            },
        ]
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
    let signals = engine_state.signals().clone();

    let definition = match input.into_value(span)? {
        Value::Nothing { .. } => {
            let provider = match call.get_flag::<String>(engine_state, stack, "provider")? {
                Some(p) => Provider::try_from(p.as_str())?,
                None => {
                    return Err(generic_error(
                        "no provider specified",
                        "Please specify a cloud provider using the '--provider' flag".to_string(),
                        None,
                    ));
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
        Value::String { val, .. } => serde_json::from_str(val.as_str())
            .map_err(|_| could_not_parse_cluster_definition_error())?,
        _ => {
            return Err(could_not_parse_cluster_definition_error());
        }
    };

    let capella = call.get_flag(engine_state, stack, "capella")?;

    debug!("Running clusters create for {:?}", definition);

    let guard = state.lock().unwrap();
    let control = guard.named_or_active_org(capella)?;
    let client = control.client();

    let project =
        guard.named_or_active_project(call.get_flag(engine_state, stack, "project")?)?;

    let org_id = find_org_id(signals.clone(), &client, span)?;
    let project_id = find_project_id(signals.clone(), project, &client, span, org_id.clone())?;

    let payload =
        serde_json::to_string(&definition).map_err(|e| serialize_error(e.to_string(), span))?;
    client
        .create_cluster(org_id, project_id, payload, signals)
        .map_err(|e| client_error_to_shell_error(e, span))?;

    Ok(PipelineData::empty())
}

fn random_cluster_name() -> String {
    let mut uuid = Uuid::new_v4().to_string();
    uuid.truncate(6);
    format!("cbshell-cluster-{}", uuid)
}

fn could_not_parse_cluster_definition_error() -> ShellError {
    generic_error(
        "Could not parse cluster definition",
        "Piped cluster definition must be a string in th format defined by the Capella v4 API. Run 'clusters create --help' for an example".to_string(),
        None
    )
}
