use crate::cli::util::cluster_identifiers_from;
use crate::state::State;

use crate::client::ManagementRequest;
use async_trait::async_trait;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue};
use nu_source::Tag;
use nu_stream::OutputStream;
use serde::Deserialize;
use std::fmt;
use std::ops::Add;
use std::sync::Arc;
use tokio::time::Instant;

pub struct Nodes {
    state: Arc<State>,
}

impl Nodes {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for Nodes {
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

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        nodes(self.state.clone(), args)
    }
}

fn nodes(state: Arc<State>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();
    let args = args.evaluate_once()?;

    let cluster_identifiers = cluster_identifiers_from(&state, &args, true)?;

    let mut nodes = vec![];
    for identifier in cluster_identifiers {
        let active_cluster = match state.clusters().get(&identifier) {
            Some(c) => c,
            None => {
                return Err(ShellError::untagged_runtime_error("Cluster not found"));
            }
        };

        let response = active_cluster.cluster().management_request(
            ManagementRequest::GetNodes,
            Instant::now().add(active_cluster.timeouts().query_timeout()),
            ctrl_c.clone(),
        )?;

        let resp: PoolInfo = match response.status() {
            200 => match serde_json::from_str(response.content()) {
                Ok(m) => m,
                Err(e) => {
                    return Err(ShellError::untagged_runtime_error(format!(
                        "Failed to decode response body {}",
                        e,
                    )));
                }
            },
            _ => {
                return Err(ShellError::untagged_runtime_error(format!(
                    "Request failed {}",
                    response.content(),
                )));
            }
        };

        let mut n = resp
            .nodes
            .into_iter()
            .map(|n| {
                let mut collected = TaggedDictBuilder::new(Tag::default());
                let services = n
                    .services
                    .iter()
                    .map(|n| format!("{}", n))
                    .collect::<Vec<_>>()
                    .join(",");

                collected.insert_value("cluster", identifier.clone());
                collected.insert_value("hostname", n.hostname);
                collected.insert_value("status", n.status);
                collected.insert_value("services", services);
                collected.insert_value("version", n.version);
                collected.insert_value("os", n.os);
                collected.insert_value("memory_total", UntaggedValue::filesize(n.memory_total));
                collected.insert_value("memory_free", UntaggedValue::filesize(n.memory_free));

                collected.into_value()
            })
            .collect::<Vec<_>>();

        nodes.append(&mut n);
    }

    Ok(nodes.into())
}

#[derive(Debug, Deserialize)]
struct PoolInfo {
    name: String,
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

#[derive(Debug, Deserialize)]
enum NodeService {
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
