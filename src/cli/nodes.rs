use crate::cli::convert_cb_error;
use crate::cli::util::cluster_identifiers_from;
use crate::state::State;

use async_trait::async_trait;
use couchbase::{GenericManagementRequest, Request};
use futures::channel::oneshot;
use nu_cli::{CommandArgs, OutputStream};
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue};
use nu_source::Tag;
use serde::Deserialize;
use std::fmt;
use std::sync::Arc;

pub struct Nodes {
    state: Arc<State>,
}

impl Nodes {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_cli::WholeStreamCommand for Nodes {
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

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        nodes(self.state.clone(), args).await
    }
}

async fn nodes(state: Arc<State>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once().await?;

    let cluster_identifiers = cluster_identifiers_from(&state, &args, true)?;

    let mut nodes = vec![];
    for identifier in cluster_identifiers {
        let core = match state.clusters().get(&identifier) {
            Some(c) => c.cluster().core(),
            None => {
                return Err(ShellError::untagged_runtime_error("Cluster not found"));
            }
        };
        let (sender, receiver) = oneshot::channel();
        let request =
            GenericManagementRequest::new(sender, "/pools/default".into(), "get".into(), None);
        core.send(Request::GenericManagementRequest(request));

        let input = match receiver.await {
            Ok(i) => i,
            Err(e) => {
                return Err(ShellError::untagged_runtime_error(format!(
                    "Error streaming result {}",
                    e
                )))
            }
        };
        let result = convert_cb_error(input)?;

        if !result.payload().is_some() {
            return Err(ShellError::untagged_runtime_error(
                "Empty response from cluster even though got 200 ok",
            ));
        }

        let payload = match result.payload() {
            Some(p) => p,
            None => {
                return Err(ShellError::untagged_runtime_error(
                    "Empty response from cluster even though got 200 ok",
                ));
            }
        };
        let resp: PoolInfo = serde_json::from_slice(payload)?;
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
        }
    }
}
