use crate::cli::cloud_json::JSONCloudUser;
use crate::cli::user_builder::UserAndMetadata;
use crate::cli::util::cluster_identifiers_from;
use crate::client::{CapellaRequest, ManagementRequest};
use crate::state::State;
use async_trait::async_trait;
use log::debug;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, TaggedDictBuilder, Value};
use nu_source::Tag;
use nu_stream::OutputStream;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

pub struct Users {
    state: Arc<Mutex<State>>,
}

impl Users {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for Users {
    fn name(&self) -> &str {
        "users"
    }

    fn signature(&self) -> Signature {
        Signature::build("users").named(
            "clusters",
            SyntaxShape::String,
            "the clusters which should be contacted",
            None,
        )
    }

    fn usage(&self) -> &str {
        "Lists all users"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        users_get_all(self.state.clone(), args)
    }
}

fn users_get_all(state: Arc<Mutex<State>>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();
    debug!("Running users get all");

    let cluster_identifiers = cluster_identifiers_from(&state, &args, true)?;
    let guard = state.lock().unwrap();

    let mut results = vec![];
    for identifier in cluster_identifiers {
        let active_cluster = match guard.clusters().get(&identifier) {
            Some(c) => c,
            None => {
                return Err(ShellError::unexpected("Cluster not found"));
            }
        };
        let mut stream: Vec<Value> = if let Some(plane) = active_cluster.capella_org() {
            let cloud = guard.capella_org_for_cluster(plane)?.client();
            let deadline = Instant::now().add(active_cluster.timeouts().management_timeout());
            let cluster_id =
                cloud.find_cluster_id(identifier.clone(), deadline.clone(), ctrl_c.clone())?;
            let response = cloud.capella_request(
                CapellaRequest::GetUsers { cluster_id },
                deadline,
                ctrl_c.clone(),
            )?;
            if response.status() != 200 {
                return Err(ShellError::unexpected(response.content()));
            }

            let users: Vec<JSONCloudUser> = serde_json::from_str(response.content())?;

            users
                .into_iter()
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
                ManagementRequest::GetUsers,
                Instant::now().add(active_cluster.timeouts().management_timeout()),
                ctrl_c.clone(),
            )?;

            let users: Vec<UserAndMetadata> = match response.status() {
                200 => match serde_json::from_str(response.content()) {
                    Ok(m) => m,
                    Err(e) => {
                        return Err(ShellError::unexpected(format!(
                            "Failed to decode response body {}",
                            e,
                        )));
                    }
                },
                _ => {
                    return Err(ShellError::unexpected(format!(
                        "Request failed {}",
                        response.content(),
                    )));
                }
            };

            users
                .into_iter()
                .map(|v| {
                    let user = v.user();
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
                    } else {
                        collected.insert_value("groups", "")
                    }
                    collected.insert_value("roles", roles.join(","));
                    if let Some(changed) = v.password_changed() {
                        collected.insert_value("password_last_changed", changed)
                    } else {
                        collected.insert_value("password_last_changed", "")
                    }
                    collected.insert_value("cluster", identifier.clone());
                    collected.into_value()
                })
                .collect()
        };

        results.append(&mut stream);
    }

    Ok(OutputStream::from(results))
}
