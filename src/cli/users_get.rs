use crate::cli::cloud_json::JSONCloudUser;
use crate::cli::user_builder::UserAndMetadata;
use crate::cli::util::cluster_identifiers_from;
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

pub struct UsersGet {
    state: Arc<Mutex<State>>,
}

impl UsersGet {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for UsersGet {
    fn name(&self) -> &str {
        "users get"
    }

    fn signature(&self) -> Signature {
        Signature::build("users get")
            .required("username", SyntaxShape::String, "the username of the user")
            .named(
                "clusters",
                SyntaxShape::String,
                "the clusters which should be contacted",
                None,
            )
    }

    fn usage(&self) -> &str {
        "Fetches a user"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        users_get(self.state.clone(), args)
    }
}

fn users_get(state: Arc<Mutex<State>>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();
    let username: String = args.req(0)?;

    debug!("Running users get {}", username);

    let cluster_identifiers = cluster_identifiers_from(&state, &args, true)?;
    let guard = state.lock().unwrap();

    let mut results = vec![];
    for identifier in cluster_identifiers {
        let active_cluster = match guard.clusters().get(&identifier) {
            Some(c) => c,
            None => {
                return Err(ShellError::untagged_runtime_error("Cluster not found"));
            }
        };

        let mut stream: Vec<Value> = if active_cluster.cloud() {
            let cloud = guard.cloud_control_pane()?.client();
            let deadline = Instant::now().add(active_cluster.timeouts().management_timeout());
            let cluster_id =
                cloud.find_cluster_id(identifier.clone(), deadline.clone(), ctrl_c.clone())?;
            let response = cloud.cloud_request(
                CloudRequest::GetUsers { cluster_id },
                deadline,
                ctrl_c.clone(),
            )?;
            if response.status() != 200 {
                return Err(ShellError::unexpected(response.content()));
            }

            let users: Vec<JSONCloudUser> = serde_json::from_str(response.content())?;

            users
                .into_iter()
                .filter(|user| user.username() == username.clone())
                .map(|user| {
                    let mut roles: Vec<String> = Vec::new();
                    for role in user.roles().iter() {
                        for name in role.names() {
                            roles.push(format!("{}[{}]", role.bucket().clone(), name));
                        }
                    }

                    let mut collected = TaggedDictBuilder::new(Tag::default());
                    collected.insert_value("username", user.username());
                    collected.insert_value("display name", "");
                    collected.insert_value("groups", "");
                    collected.insert_value("roles", roles.join(","));
                    collected.insert_value("password_last_changed", "");
                    collected.insert_value("cluster", identifier.clone());
                    collected.into_value()
                })
                .collect()
        } else {
            let response = active_cluster.cluster().http_client().management_request(
                ManagementRequest::GetUser {
                    username: username.clone(),
                },
                Instant::now().add(active_cluster.timeouts().management_timeout()),
                ctrl_c.clone(),
            )?;

            let user_and_meta: UserAndMetadata = match response.status() {
                200 => match serde_json::from_str(response.content()) {
                    Ok(m) => m,
                    Err(e) => {
                        return Err(ShellError::untagged_runtime_error(format!(
                            "Failed to decode response body {}",
                            e,
                        )));
                    }
                },
                _ => {
                    return Err(ShellError::untagged_runtime_error(format!(
                        "Request failed {}",
                        response.content(),
                    )));
                }
            };

            let user = user_and_meta.user();
            let roles: Vec<String> = user
                .roles()
                .iter()
                .map(|r| match r.bucket() {
                    Some(b) => format!("{}[{}]", r.name(), b),
                    None => r.name().to_string(),
                })
                .collect();

            let mut collected = TaggedDictBuilder::new(Tag::default());
            collected.insert_value("username", user.username());
            collected.insert_value("display name", user.display_name().unwrap_or_default());
            if let Some(groups) = user.groups() {
                collected.insert_value("groups", groups.join(","))
            }
            collected.insert_value("roles", roles.join(","));
            if let Some(changed) = user_and_meta.password_changed() {
                collected.insert_value("password_last_changed", changed)
            }
            collected.insert_value("cluster", identifier.clone());

            vec![collected.into_value()]
        };

        results.append(&mut stream);
    }

    Ok(OutputStream::from(results))
}
