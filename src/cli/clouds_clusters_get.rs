use crate::cli::cloud_json::JSONCloudCluster;
use crate::cli::util::find_cloud_cluster_id;
use crate::client::CloudRequest;
use crate::state::State;
use async_trait::async_trait;
use log::debug;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, TaggedDictBuilder};
use nu_source::Tag;
use nu_stream::OutputStream;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

pub struct CloudsClustersGet {
    state: Arc<Mutex<State>>,
}

impl CloudsClustersGet {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for CloudsClustersGet {
    fn name(&self) -> &str {
        "clouds clusters-get"
    }

    fn signature(&self) -> Signature {
        Signature::build("clouds clusters-get").required(
            "name",
            SyntaxShape::String,
            "the name of the cluster",
        )
    }

    fn usage(&self) -> &str {
        "Gets a cloud cluster"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        clusters_get(self.state.clone(), args)
    }
}

fn clusters_get(state: Arc<Mutex<State>>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();
    let name: String = args.req(0)?;

    debug!("Running clouds clusters get for {}", &name);

    let guard = state.lock().unwrap();
    let control = guard.active_cloud_org()?;
    let client = control.client();

    let deadline = Instant::now().add(control.timeout());
    let cluster_id = find_cloud_cluster_id(ctrl_c.clone(), name, &client, deadline)?;
    let response =
        client.cloud_request(CloudRequest::GetCluster { cluster_id }, deadline, ctrl_c)?;
    if response.status() != 200 {
        return Err(ShellError::unexpected(response.content().to_string()));
    };
    let cluster: JSONCloudCluster = serde_json::from_str(response.content())?;

    let mut collected = TaggedDictBuilder::new(Tag::default());
    collected.insert_value("name", cluster.name());
    collected.insert_value("id", cluster.id());
    collected.insert_value("status", cluster.status());
    collected.insert_value("endpoint_urls", cluster.endpoints_url().join(","));
    collected.insert_value(
        "endpoint_srv",
        cluster.endpoints_srv().unwrap_or_else(|| "".to_string()),
    );
    collected.insert_value("version", cluster.version_name());
    collected.insert_value("cloud_id", cluster.cloud_id());
    collected.insert_value("tenant_id", cluster.tenant_id());
    collected.insert_value("project_id", cluster.project_id());

    Ok(OutputStream::from(vec![collected.into_value()]))
}
