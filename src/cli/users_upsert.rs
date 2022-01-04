use crate::cli::cloud_json::{JSONCloudCreateUserRequest, JSONCloudUser, JSONCloudUserRoles};
use crate::cli::util::cluster_identifiers_from;
use crate::client::{CapellaRequest, ManagementRequest};
use crate::state::State;
use async_trait::async_trait;
use log::debug;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_stream::OutputStream;
use std::collections::HashMap;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

pub struct UsersUpsert {
    state: Arc<Mutex<State>>,
}

impl UsersUpsert {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for UsersUpsert {
    fn name(&self) -> &str {
        "users upsert"
    }

    fn signature(&self) -> Signature {
        Signature::build("users upsert")
            .required("username", SyntaxShape::String, "the username of the user")
            .required(
                "roles",
                SyntaxShape::String,
                "the roles for the user <role_name[bucket_name]>",
            )
            .named(
                "password",
                SyntaxShape::String,
                "the password for the user",
                None,
            )
            .named(
                "display_name",
                SyntaxShape::String,
                "the display name for the user",
                None,
            )
            .named(
                "groups",
                SyntaxShape::String,
                "the group names for the user",
                None,
            )
            .named(
                "clusters",
                SyntaxShape::String,
                "the clusters which should be contacted",
                None,
            )
    }

    fn usage(&self) -> &str {
        "Upserts a user"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        users_upsert(self.state.clone(), args)
    }
}

fn users_upsert(state: Arc<Mutex<State>>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();
    let username: String = args.req(0)?;
    let roles: String = args.req(1)?;
    let password = args.get_flag("password")?;
    let display_name = args.get_flag("display_name")?;
    let groups = args.get_flag("groups")?;

    debug!("Running users upsert for user {}", &username);

    let cluster_identifiers = cluster_identifiers_from(&state, &args, true)?;
    let guard = state.lock().unwrap();

    for identifier in cluster_identifiers {
        let active_cluster = match guard.clusters().get(&identifier) {
            Some(c) => c,
            None => {
                return Err(ShellError::unexpected("Cluster not found"));
            }
        };
        let response = if let Some(plane) = active_cluster.capella_org() {
            let mut bucket_roles_map: HashMap<&str, Vec<&str>> = HashMap::new();
            for role in roles.split(',') {
                let split_role = role.split_once("[");
                let bucket_role = if let Some(mut split) = split_role {
                    split.1 = split.1.strip_suffix("]").unwrap_or(split.1);
                    (split.0, split.1)
                } else {
                    (role, "*")
                };
                if let Some(br) = bucket_roles_map.get_mut(bucket_role.1) {
                    br.push(bucket_role.0);
                } else {
                    bucket_roles_map.insert(bucket_role.1, vec![bucket_role.0]);
                }
            }

            let mut bucket_roles = Vec::new();
            let mut all_access_roles: Vec<String> = Vec::new();
            for (bucket, roles) in bucket_roles_map {
                if bucket == "*" {
                    all_access_roles.push(roles.iter().map(|r| r.to_string()).collect());
                } else {
                    bucket_roles.push(JSONCloudUserRoles::new(
                        bucket.to_string(),
                        roles.iter().map(|r| r.to_string()).collect(),
                    ));
                }
            }

            let all_access_role = if all_access_roles.len() == 0 {
                "".to_string()
            } else if all_access_roles.len() == 1 {
                all_access_roles[0].clone()
            } else {
                return Err(ShellError::unexpected(
                    "Users with cluster scoped permissions can only be assigned one role",
                ));
            };

            let cloud = guard.capella_org_for_cluster(plane)?.client();
            let deadline = Instant::now().add(active_cluster.timeouts().management_timeout());
            let cluster_id = cloud.find_cluster_id(identifier.clone(), deadline, ctrl_c.clone())?;
            let response = cloud.capella_request(
                CapellaRequest::GetUsers {
                    cluster_id: cluster_id.clone(),
                },
                deadline,
                ctrl_c.clone(),
            )?;
            if response.status() != 200 {
                return Err(ShellError::unexpected(response.content()));
            }

            let users: Vec<JSONCloudUser> = serde_json::from_str(response.content())?;

            let mut user_exists = false;
            for u in users {
                if u.username() == username.clone() {
                    user_exists = true;
                }
            }

            if user_exists {
                // Update
                let user = JSONCloudCreateUserRequest::new(
                    username.clone(),
                    password.clone().unwrap_or_default(),
                    bucket_roles,
                    all_access_role,
                );
                cloud.capella_request(
                    CapellaRequest::UpdateUser {
                        cluster_id,
                        payload: serde_json::to_string(&user)?,
                        username: username.clone(),
                    },
                    deadline,
                    ctrl_c.clone(),
                )?
            } else {
                // Create
                let pass = match password.clone() {
                    Some(p) => p,
                    None => {
                        return Err(ShellError::unexpected(
                            "Capella database user does not exist, password must be set",
                        ));
                    }
                };

                let user = JSONCloudCreateUserRequest::new(
                    username.clone(),
                    pass,
                    bucket_roles,
                    all_access_role,
                );
                cloud.capella_request(
                    CapellaRequest::CreateUser {
                        cluster_id,
                        payload: serde_json::to_string(&user)?,
                    },
                    deadline,
                    ctrl_c.clone(),
                )?
            }
        } else {
            let form = &[
                ("name", display_name.clone()),
                ("groups", groups.clone()),
                ("roles", Some(roles.clone())),
                ("password", password.clone()),
            ];
            let payload = serde_urlencoded::to_string(form).unwrap();

            active_cluster.cluster().http_client().management_request(
                ManagementRequest::UpsertUser {
                    username: username.clone(),
                    payload,
                },
                Instant::now().add(active_cluster.timeouts().management_timeout()),
                ctrl_c.clone(),
            )?
        };

        match response.status() {
            200 => {}
            201 => {}
            202 => {}
            204 => {}
            _ => {
                return Err(ShellError::unexpected(response.content()));
            }
        }
    }

    Ok(OutputStream::empty())
}
