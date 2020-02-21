use couchbase::Cluster;
use futures::executor::block_on;
use nu::{CommandArgs, CommandRegistry, OutputStream};
use nu_errors::ShellError;
use nu_protocol::{Signature, TaggedDictBuilder, UntaggedValue};
use nu_source::Tag;
use serde::Deserialize;
use std::sync::Arc;

pub struct Nodes {
    _cluster: Arc<Cluster>,
}

impl Nodes {
    pub fn new(cluster: Arc<Cluster>) -> Self {
        Self { _cluster: cluster }
    }
}

impl nu::WholeStreamCommand for Nodes {
    fn name(&self) -> &str {
        "nodes"
    }

    fn signature(&self) -> Signature {
        Signature::build("nodes")
    }

    fn usage(&self) -> &str {
        "Lists all nodes of the connected cluster"
    }

    fn run(
        &self,
        _args: CommandArgs,
        _registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        block_on(nodes())
    }
}

async fn nodes() -> Result<OutputStream, ShellError> {
    let client = reqwest::Client::new();

    let resp = client
        .get("http://localhost:8091/pools/default")
        .basic_auth("Administrator", Some("password"))
        .send()
        .await
        .unwrap()
        .json::<PoolInfo>()
        .await
        .unwrap();

    let nodes = resp
        .nodes
        .into_iter()
        .map(|n| {
            let mut collected = TaggedDictBuilder::new(Tag::default());
            collected.insert_value("hostname", n.hostname);
            collected.insert_value("status", n.status);
            collected.insert_value(
                "memory_total",
                UntaggedValue::bytes(n.memory_total).into_untagged_value(),
            );
            collected.insert_value(
                "memory_free",
                UntaggedValue::bytes(n.memory_free).into_untagged_value(),
            );

            collected.into_value()
        })
        .collect::<Vec<_>>();

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
}
