use crate::cli::cloud_json::JSONCloudClustersSummaries;
use crate::client::CloudRequest;
use crate::state::State;
use async_trait::async_trait;
use log::debug;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, TaggedDictBuilder, UntaggedValue};
use nu_source::Tag;
use nu_stream::OutputStream;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

pub struct CloudsClusters {
    state: Arc<Mutex<State>>,
}

impl CloudsClusters {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for CloudsClusters {
    fn name(&self) -> &str {
        "clouds clusters"
    }

    fn signature(&self) -> Signature {
        Signature::build("clouds clusters")
    }

    fn usage(&self) -> &str {
        "Lists all cloud clusters"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        cloud_clusters(self.state.clone(), args)
    }
}

fn cloud_clusters(state: Arc<Mutex<State>>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();

    debug!("Running clouds clusters");

    let guard = state.lock().unwrap();
    let control = guard.active_cloud_org()?;
    let client = control.client();
    let response = client.cloud_request(
        CloudRequest::GetClusters {},
        Instant::now().add(control.timeout()),
        ctrl_c,
    )?;
    if response.status() != 200 {
        return Err(ShellError::unexpected(response.content().to_string()));
    };

    let content: JSONCloudClustersSummaries = serde_json::from_str(response.content())?;

    let mut results = vec![];
    for cluster in content.items() {
        let mut collected = TaggedDictBuilder::new(Tag::default());
        collected.insert_value("name", cluster.name());
        collected.insert_value("id", cluster.id());
        collected.insert_value("services", cluster.services().join(","));
        collected.insert_value("nodes", UntaggedValue::int(cluster.nodes()));
        results.push(collected.into_value())
    }

    Ok(OutputStream::from(results))
}
