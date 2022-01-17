use crate::cli::cloud_json::JSONCloudClustersSummariesV3;
use crate::client::CapellaRequest;
use crate::state::State;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, TaggedDictBuilder};
use nu_source::Tag;
use nu_stream::OutputStream;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

pub struct Clusters {
    state: Arc<Mutex<State>>,
}

impl Clusters {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl nu_engine::WholeStreamCommand for Clusters {
    fn name(&self) -> &str {
        "clusters"
    }

    fn signature(&self) -> Signature {
        Signature::build("clusters").named(
            "capella",
            SyntaxShape::String,
            "the Capella organization to use",
            None,
        )
    }

    fn usage(&self) -> &str {
        "Lists all clusters on the active Capella organisation"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        clusters(args, self.state.clone())
    }
}

fn clusters(args: CommandArgs, state: Arc<Mutex<State>>) -> Result<OutputStream, ShellError> {
    let capella = args.get_flag("capella")?;

    let guard = state.lock().unwrap();

    let ctrl_c = args.ctrl_c();
    let control = if let Some(c) = capella {
        guard.capella_org_for_cluster(c)
    } else {
        guard.active_capella_org()
    }?;
    let client = control.client();

    let response = client.capella_request(
        CapellaRequest::GetClustersV3 {},
        Instant::now().add(control.timeout()),
        ctrl_c,
    )?;
    if response.status() != 200 {
        return Err(ShellError::unexpected(response.content().to_string()));
    };

    let content: JSONCloudClustersSummariesV3 = serde_json::from_str(response.content())?;

    let mut results = vec![];
    for cluster in content.items() {
        let mut collected = TaggedDictBuilder::new(Tag::default());
        collected.insert_value("name", cluster.name());
        collected.insert_value("id", cluster.id());
        collected.insert_value("cloud_id", cluster.cloud_id());
        collected.insert_value("project_id", cluster.project_id());
        collected.insert_value("environment", cluster.environment());
        collected.insert_value("tenant_id", content.tenant_id());
        results.push(collected.into_value())
    }

    Ok(OutputStream::from(results))
}
