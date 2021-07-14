use crate::cli::util::find_project_id;
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
            "the name of the project",
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
    let control = guard.active_cloud_org()?;
    let client = control.client();
    let deadline = Instant::now().add(control.timeout());
    let project_id = find_project_id(ctrl_c.clone(), name, &client, deadline)?;

    let response = client.cloud_request(
        CloudRequest::DeleteProject {
            project_id: project_id.to_string(),
        },
        deadline,
        ctrl_c,
    )?;
    if response.status() != 204 {
        return Err(ShellError::unexpected(response.content().to_string()));
    };

    Ok(OutputStream::empty())
}
