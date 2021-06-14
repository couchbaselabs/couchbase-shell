use crate::cli::cloud_json::JSONCloudUser;
use crate::cli::user_builder::UserAndMetadata;
use crate::client::{CloudRequest, ManagementRequest};
use crate::state::State;
use async_trait::async_trait;
use log::debug;
use nu_cli::TaggedDictBuilder;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, Value};
use nu_source::Tag;
use nu_stream::OutputStream;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

pub struct UsersDrop {
    state: Arc<Mutex<State>>,
}

impl UsersDrop {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for UsersDrop {
    fn name(&self) -> &str {
        "users drop"
    }

    fn signature(&self) -> Signature {
        Signature::build("users drop").required(
            "username",
            SyntaxShape::String,
            "the username of the user",
        )
    }

    fn usage(&self) -> &str {
        "Deletes a user"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        users_drop(self.state.clone(), args)
    }
}

fn users_drop(state: Arc<Mutex<State>>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();
    let args = args.evaluate_once()?;
    let username = args.nth(0).expect("need username").as_string()?;

    debug!("Running users drop {}", username);

    let guard = state.lock().unwrap();
    let active_cluster = guard.active_cluster();

    let response = if let Some(c) = active_cluster.cloud() {
        let identifier = guard.active();
        let cloud = guard.cloud_for_cluster(c)?.cloud();
        let cluster_id = cloud.find_cluster_id(
            identifier.clone(),
            Instant::now().add(active_cluster.timeouts().query_timeout()),
            ctrl_c.clone(),
        )?;
        cloud.cloud_request(
            CloudRequest::DeleteUser {
                cluster_id,
                username,
            },
            Instant::now().add(active_cluster.timeouts().query_timeout()),
            ctrl_c.clone(),
        )?
    } else {
        active_cluster.cluster().http_client().management_request(
            ManagementRequest::DropUser { username },
            Instant::now().add(active_cluster.timeouts().query_timeout()),
            ctrl_c,
        )?
    };

    match response.status() {
        200 => Ok(OutputStream::empty()),
        204 => Ok(OutputStream::empty()),
        _ => Err(ShellError::untagged_runtime_error(
            response.content().to_string(),
        )),
    }
}
