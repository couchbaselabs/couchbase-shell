use crate::cli::util::cluster_identifiers_from;
use crate::state::State;

use crate::cli::cloud_json::JSONCloudClusterHealthResponse;
use crate::client::{CloudRequest, ManagementRequest};
use async_trait::async_trait;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue};
use nu_source::Tag;
use nu_stream::OutputStream;
use serde::Deserialize;
use std::fmt;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

pub struct Nodes {
    state: Arc<Mutex<State>>,
}

impl Nodes {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
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

fn nodes(state: Arc<Mutex<State>>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();

    let cluster_identifiers = cluster_identifiers_from(&state, &args, true)?;

    let guard = state.lock().unwrap();
    let mut nodes = vec![];
    for identifier in cluster_identifiers {
        let active_cluster = match guard.clusters().get(&identifier) {
            Some(c) => c,
            None => {
                return Err(ShellError::unexpected("Cluster not found"));
            }
        };
        if let Some(plane) = active_cluster.cloud_org() {
            let cloud = guard.cloud_org_for_cluster(plane)?.client();
            let deadline = Instant::now().add(active_cluster.timeouts().management_timeout());
            let cluster_id =
                cloud.find_cluster_id(identifier.clone(), deadline.clone(), ctrl_c.clone())?;
            let response = cloud.cloud_request(
                CloudRequest::GetClusterHealth { cluster_id },
                deadline,
                ctrl_c.clone(),
            )?;
            if response.status() != 200 {
                return Err(ShellError::unexpected(response.content()));
            }

            dbg!(response.content());
            let resp: JSONCloudClusterHealthResponse = serde_json::from_str(response.content())?;

            let mut n = resp
                .nodes()
                .nodes()
                .into_iter()
                .map(|n| {
                    let mut collected = TaggedDictBuilder::new(Tag::default());
                    let services = n
                        .services()
                        .iter()
                        .map(|n| format!("{}", n))
                        .collect::<Vec<_>>()
                        .join(",");

                    collected.insert_value("cluster", identifier.clone());
                    collected.insert_value("hostname", n.name());
                    collected.insert_value("status", n.status());
                    collected.insert_value("services", services);
                    collected.insert_value("version", "");
                    collected.insert_value("os", "");
                    collected.insert_value("memory_total", "");
                    collected.insert_value("memory_free", "");
                    collected.insert_value("cloud", true);

                    collected.into_value()
                })
                .collect::<Vec<_>>();

            nodes.append(&mut n);
        } else {
            let response = active_cluster.cluster().http_client().management_request(
                ManagementRequest::GetNodes,
                Instant::now().add(active_cluster.timeouts().management_timeout()),
                ctrl_c.clone(),
            )?;

            let resp: PoolInfo = match response.status() {
                200 => match serde_json::from_str(response.content()) {
                    Ok(m) => m,
                    Err(e) => {
                        return Err(ShellError::unexpected(format!(
                            "Failed to decode response body {}",
                            e,
                        )));
                    }
                },
                _ => {
                    return Err(ShellError::unexpected(format!(
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
                    collected.insert_value("cloud", false);

                    collected.into_value()
                })
                .collect::<Vec<_>>();

            nodes.append(&mut n);
        }
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
