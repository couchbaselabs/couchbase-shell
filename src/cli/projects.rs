use crate::cli::cloud_json::JSONCloudsProjectsResponse;
use crate::client::CloudRequest;
use crate::state::State;
use async_trait::async_trait;
use log::debug;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, TaggedDictBuilder};
use nu_source::Tag;
use nu_stream::OutputStream;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

pub struct Projects {
    state: Arc<Mutex<State>>,
}

impl Projects {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for Projects {
    fn name(&self) -> &str {
        "projects"
    }

    fn signature(&self) -> Signature {
        Signature::build("projects")
    }

    fn usage(&self) -> &str {
        "Lists all cloud projects"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        projects(self.state.clone(), args)
    }
}

fn projects(state: Arc<Mutex<State>>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();

    debug!("Running projects");

    let guard = state.lock().unwrap();
    let control = guard.active_cloud_org()?;
    let client = control.client();
    let response = client.cloud_request(
        CloudRequest::GetProjects {},
        Instant::now().add(control.timeout()),
        ctrl_c,
    )?;
    if response.status() != 200 {
        return Err(ShellError::unexpected(response.content().to_string()));
    };

    let content: JSONCloudsProjectsResponse = serde_json::from_str(response.content())?;

    let mut results = vec![];
    for project in content.items() {
        let mut collected = TaggedDictBuilder::new(Tag::default());
        collected.insert_value("name", project.name());
        collected.insert_value("id", project.id());
        results.push(collected.into_value())
    }

    Ok(OutputStream::from(results))
}
