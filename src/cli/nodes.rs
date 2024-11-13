use crate::cli::util::{cluster_identifiers_from, get_active_cluster, NuValueMap};
use crate::state::State;

use crate::client::ManagementRequest;
use serde::Deserialize;
use std::fmt;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

use crate::cli::error::{
    client_error_to_shell_error, serialize_error, unexpected_status_code_error,
};
use crate::remote_cluster::RemoteClusterType::Provisioned;
use nu_engine::command_prelude::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value};

#[derive(Clone)]
pub struct Nodes {
    state: Arc<Mutex<State>>,
}

impl Nodes {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for Nodes {
    fn name(&self) -> &str {
        "nodes"
    }

    fn signature(&self) -> Signature {
        Signature::build("nodes").named(
            "clusters",
            SyntaxShape::String,
            "the clusters which should be contacted",
            None,
        )
    }

    fn usage(&self) -> &str {
        "Lists all nodes of the connected cluster"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        nodes(self.state.clone(), engine_state, stack, call, input)
    }
}

fn nodes(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let signals = engine_state.signals().clone();
    let span = call.head;

    let cluster_identifiers = cluster_identifiers_from(engine_state, stack, &state, call, true)?;

    let guard = state.lock().unwrap();
    let mut nodes = vec![];
    for identifier in cluster_identifiers {
        let active_cluster = get_active_cluster(identifier.clone(), &guard, span)?;
        let response = active_cluster
            .cluster()
            .http_client()
            .management_request(
                ManagementRequest::GetNodes,
                Instant::now().add(active_cluster.timeouts().management_timeout()),
                signals.clone(),
            )
            .map_err(|e| client_error_to_shell_error(e, span))?;
        if response.status() != 200 {
            return Err(unexpected_status_code_error(
                response.status(),
                response.content()?,
                span,
            ));
        }

        let resp: PoolInfo = match response.status() {
            200 => serde_json::from_str(&response.content()?)
                .map_err(|e| serialize_error(e.to_string(), call.span()))?,
            _ => {
                return Err(unexpected_status_code_error(
                    response.status(),
                    response.content()?,
                    call.span(),
                ));
            }
        };

        let mut n = resp
            .nodes
            .into_iter()
            .map(|n| {
                let mut collected = NuValueMap::default();
                let services = n
                    .services
                    .iter()
                    .map(|n| format!("{}", n))
                    .collect::<Vec<_>>()
                    .join(",");

                collected.add_string("cluster", identifier.clone(), call.head);
                collected.add_string("hostname", n.hostname, call.head);
                collected.add_string("status", n.status, call.head);
                collected.add_string("services", services, call.head);
                collected.add_string("version", n.version, call.head);
                collected.add_string("os", n.os, call.head);
                collected.add_i64("memory_total", n.memory_total as i64, call.head);
                collected.add_i64("memory_free", n.memory_free as i64, call.head);
                collected.add_bool(
                    "capella",
                    active_cluster.cluster_type() == Provisioned,
                    call.head,
                );

                collected.into_value(call.head)
            })
            .collect::<Vec<_>>();

        nodes.append(&mut n);
    }

    Ok(Value::List {
        vals: nodes,
        internal_span: call.head,
    }
    .into_pipeline_data())
}

#[derive(Debug, Deserialize)]
struct PoolInfo {
    nodes: Vec<NodeInfo>,
}

#[derive(Debug, Deserialize)]
struct NodeInfo {
    hostname: String,
    status: String,
    #[serde(rename = "memoryTotal")]
    memory_total: u64,
    #[serde(rename = "memoryFree")]
    memory_free: u64,
    services: Vec<NodeService>,
    version: String,
    os: String,
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) enum NodeService {
    #[serde(rename = "cbas")]
    Analytics,
    #[serde(rename = "eventing")]
    Eventing,
    #[serde(rename = "fts")]
    Search,
    #[serde(rename = "n1ql")]
    Query,
    #[serde(rename = "index")]
    Indexing,
    #[serde(rename = "kv")]
    KeyValue,
    #[serde(rename = "backup")]
    Backup,
}

impl fmt::Display for NodeService {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            NodeService::Analytics => write!(f, "analytics"),
            NodeService::Eventing => write!(f, "eventing"),
            NodeService::Search => write!(f, "search"),
            NodeService::Query => write!(f, "query"),
            NodeService::Indexing => write!(f, "indexing"),
            NodeService::KeyValue => write!(f, "kv"),
            NodeService::Backup => write!(f, "backup"),
        }
    }
}
