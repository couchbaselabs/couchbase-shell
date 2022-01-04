use crate::cli::cloud_json::{JSONCloudCluster, JSONCloudClusterV3};
use crate::cli::util::{find_capella_cluster_id_hosted, find_capella_cluster_id_vpc};
use crate::client::CapellaRequest;
use crate::state::{CapellaEnvironment, State};
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

pub struct ClustersGet {
    state: Arc<Mutex<State>>,
}

impl ClustersGet {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for ClustersGet {
    fn name(&self) -> &str {
        "clusters get"
    }

    fn signature(&self) -> Signature {
        Signature::build("clusters get")
            .required("name", SyntaxShape::String, "the name of the cluster")
            .named(
                "capella",
                SyntaxShape::String,
                "the Capella organization to use",
                None,
            )
    }

    fn usage(&self) -> &str {
        "Gets a cluster from the active Capella organization"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        clusters_get(self.state.clone(), args)
    }
}

fn clusters_get(state: Arc<Mutex<State>>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();
    let name: String = args.req(0)?;
    let capella = args.get_flag("capella")?;

    debug!("Running clusters get for {}", &name);

    let guard = state.lock().unwrap();
    let control = if let Some(c) = capella {
        guard.capella_org_for_cluster(c)
    } else {
        guard.active_capella_org()
    }?;
    let client = control.client();

    let deadline = Instant::now().add(control.timeout());
    if control.environment() == CapellaEnvironment::Hosted {
        let cluster_id = find_capella_cluster_id_hosted(ctrl_c.clone(), name, &client, deadline)?;

        let response = client.capella_request(
            CapellaRequest::GetClusterV3 { cluster_id },
            deadline,
            ctrl_c,
        )?;
        if response.status() != 200 {
            return Err(ShellError::unexpected(response.content().to_string()));
        };
        let cluster: JSONCloudClusterV3 = serde_json::from_str(response.content())?;

        let mut collected = TaggedDictBuilder::new(Tag::default());
        collected.insert_value("name", cluster.name());
        collected.insert_value("id", cluster.id());
        collected.insert_value("status", cluster.status());
        collected.insert_value(
            "endpoint_srv",
            cluster.endpoints_srv().unwrap_or_else(|| "".to_string()),
        );
        collected.insert_value("version", cluster.version_name());
        collected.insert_value("tenant_id", cluster.tenant_id());
        collected.insert_value("project_id", cluster.project_id());
        collected.insert_value("environment", cluster.environment());

        return Ok(OutputStream::from(vec![collected.into_value()]));
    }

    let cluster_id = find_capella_cluster_id_vpc(ctrl_c.clone(), name, &client, deadline)?;

    let response =
        client.capella_request(CapellaRequest::GetCluster { cluster_id }, deadline, ctrl_c)?;
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
