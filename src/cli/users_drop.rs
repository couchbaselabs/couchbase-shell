use crate::cli::buckets_create::collected_value_from_error_string;
use crate::cli::util::cluster_identifiers_from;
use crate::client::{CloudRequest, ManagementRequest};
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
        Signature::build("users drop")
            .required("username", SyntaxShape::String, "the username of the user")
            .named(
                "clusters",
                SyntaxShape::String,
                "the clusters which should be contacted",
                None,
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
    let username: String = args.req(0)?;

    debug!("Running users drop {}", username);

    let cluster_identifiers = cluster_identifiers_from(&state, &args, true)?;
    let guard = state.lock().unwrap();

    let mut results = vec![];
    for identifier in cluster_identifiers {
        let active_cluster = match guard.clusters().get(&identifier) {
            Some(c) => c,
            None => {
                results.push(collected_value_from_error_string(
                    identifier.clone(),
                    "Cluster not found",
                ));
                continue;
            }
        };
        let response = if let Some(plane) = active_cluster.cloud_org() {
            let cloud = guard.cloud_org_for_cluster(plane)?.client();
            let deadline = Instant::now().add(active_cluster.timeouts().management_timeout());
            let cluster_id =
                cloud.find_cluster_id(identifier.clone(), deadline.clone(), ctrl_c.clone())?;
            cloud.cloud_request(
                CloudRequest::DeleteUser {
                    cluster_id,
                    username: username.clone(),
                },
                deadline,
                ctrl_c.clone(),
            )?
        } else {
            active_cluster.cluster().http_client().management_request(
                ManagementRequest::DropUser {
                    username: username.clone(),
                },
                Instant::now().add(active_cluster.timeouts().management_timeout()),
                ctrl_c.clone(),
            )?
        };

        match response.status() {
            200 => {}
            204 => {}
            _ => {
                results.push(collected_value_from_error_string(
                    identifier.clone(),
                    response.content(),
                ));
            }
        }
    }

    Ok(OutputStream::from(results))
}
