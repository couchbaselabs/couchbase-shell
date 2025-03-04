use crate::cli::util::{find_org_id, find_project_id};
use crate::cli::{client_error_to_shell_error, generic_error, serialize_error};
use crate::client::cloud_json::ColumnarClusterCreateRequest;
use crate::state::State;
use log::{debug, info};
use nu_engine::command_prelude::Call;
use nu_engine::CallExt;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, PipelineData, ShellError, Signature, SyntaxShape, Value};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Clone)]
pub struct ColumnarClustersCreate {
    state: Arc<Mutex<State>>,
}

impl ColumnarClustersCreate {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for ColumnarClustersCreate {
    fn name(&self) -> &str {
        "columnar clusters create"
    }

    fn signature(&self) -> Signature {
        Signature::build("columnar clusters create")
            .named("name", SyntaxShape::String, "the name of the cluster", None)
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
        "Creates a new Columnar analytics cluster on the active Capella organization"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        columnar_clusters_create(self.state.clone(), engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Create a Columnar analytics cluster from a saved definition",
                example: "cat Columnar-def.json | columnar clusters create",
                result: None,
            },
            Example {
                description: "Create a 3 node Columnar analytics cluster",
                example: "columnar clusters create --nodes 3 --name myAnalytics",
                result: None,
            },
        ]
    }
}

fn columnar_clusters_create(
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
            ColumnarClusterCreateRequest::new(name, nodes)
        }
        Value::String { val, .. } => serde_json::from_str(val.as_str())
            .map_err(|_| could_not_parse_columnar_definition_error())?,
        _ => {
            return Err(could_not_parse_columnar_definition_error());
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
        .create_columnar_cluster(org_id, project_id, payload, signals)
        .map_err(|e| client_error_to_shell_error(e, span))?;

    Ok(PipelineData::empty())
}

fn random_cluster_name() -> String {
    let mut uuid = Uuid::new_v4().to_string();
    uuid.truncate(6);
    format!("cbshell-cluster-{}", uuid)
}

fn could_not_parse_columnar_definition_error() -> ShellError {
    generic_error(
        "Could not parse Columnar cluster definition",
        "Piped cluster definition must be a string in th format defined by the Capella v4 API. Run 'columnar clusters create --help' for an example".to_string(),
        None
    )
}
