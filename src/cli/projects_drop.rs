use crate::cli::cloud_json::JSONCloudsProjectsResponse;
use crate::client::CloudRequest;
use crate::state::State;
use async_trait::async_trait;
use log::debug;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_stream::OutputStream;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

pub struct ProjectsDrop {
    state: Arc<Mutex<State>>,
}

impl ProjectsDrop {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for ProjectsDrop {
    fn name(&self) -> &str {
        "projects drop"
    }

    fn signature(&self) -> Signature {
        Signature::build("projects drop").required(
            "name",
            SyntaxShape::String,
            "The name of the project",
        )
    }

    fn usage(&self) -> &str {
        "Deletes a cloud project"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        projects_create(self.state.clone(), args)
    }
}

fn projects_create(
    state: Arc<Mutex<State>>,
    args: CommandArgs,
) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();
    let name: String = args.req(0)?;

    debug!("Running projects drop for {}", &name);

    let guard = state.lock().unwrap();
    let control = guard.active_cloud_control_plane()?;
    let client = control.client();
    let response = client.cloud_request(
        CloudRequest::GetProjects {},
        Instant::now().add(control.timeout()),
        ctrl_c.clone(),
    )?;
    if response.status() != 200 {
        return Err(ShellError::untagged_runtime_error(
            response.content().to_string(),
        ));
    };
    let content: JSONCloudsProjectsResponse = serde_json::from_str(response.content())?;

    let mut project = None;
    for p in content.items() {
        if p.name() == name.clone() {
            project = Some(p);
        }
    }

    if project.is_none() {
        return Err(ShellError::unexpected("Project could not be found"));
    }

    let response = client.cloud_request(
        CloudRequest::DeleteProject {
            project_id: project.unwrap().id().to_string(),
        },
        Instant::now().add(control.timeout()),
        ctrl_c,
    )?;
    if response.status() != 204 {
        return Err(ShellError::untagged_runtime_error(
            response.content().to_string(),
        ));
    };

    Ok(OutputStream::empty())
}
