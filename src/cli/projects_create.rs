use crate::cli::cloud_json::JSONCloudCreateProjectRequest;
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

pub struct ProjectsCreate {
    state: Arc<Mutex<State>>,
}

impl ProjectsCreate {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for ProjectsCreate {
    fn name(&self) -> &str {
        "projects create"
    }

    fn signature(&self) -> Signature {
        Signature::build("projects create").required(
            "name",
            SyntaxShape::String,
            "The name of the project",
        )
    }

    fn usage(&self) -> &str {
        "Creates a new cloud project"
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

    debug!("Running projects create for {}", &name);

    let guard = state.lock().unwrap();
    let control = guard.active_cloud_control_plane()?;
    let client = control.client();
    let project = JSONCloudCreateProjectRequest::new(name);
    let response = client.cloud_request(
        CloudRequest::CreateProject {
            payload: serde_json::to_string(&project)?,
        },
        Instant::now().add(control.timeout()),
        ctrl_c,
    )?;
    if response.status() != 201 {
        return Err(ShellError::untagged_runtime_error(
            response.content().to_string(),
        ));
    };

    Ok(OutputStream::empty())
}
