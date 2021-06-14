use crate::cli::cloud_json::{JSONCloudCreateUserRequest, JSONCloudUser, JSONCloudUserRoles};
use crate::cli::util::arg_as;
use crate::client::{CloudRequest, ManagementRequest};
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
            .required_named(
                "username",
                SyntaxShape::String,
                "the username of the user",
                None,
            )
            .required_named(
                "roles",
                SyntaxShape::String,
                "the roles for the user <role_name[bucket_name]>",
                None,
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
    let args = args.evaluate_once()?;
    let username = arg_as(&args, "username", |v| v.as_string())?.unwrap();
    let roles = arg_as(&args, "roles", |v| v.as_string())?.unwrap();
    let password = arg_as(&args, "password", |v| v.as_string())?;
    let display_name = arg_as(&args, "display_name", |v| v.as_string())?;
    let groups = arg_as(&args, "groups", |v| v.as_string())?;

    debug!("Running users upsert for user {}", &username);

    let guard = state.lock().unwrap();
    let active_cluster = guard.active_cluster();

    let response = if let Some(c) = active_cluster.cloud() {
        let mut bucket_roles_map: HashMap<&str, Vec<&str>> = HashMap::new();
        for role in roles.split(",") {
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
        let mut all_access_roles = Vec::new();
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

        let identifier = guard.active();
        let cloud = guard.cloud_for_cluster(c)?.cloud();
        let cluster_id = cloud.find_cluster_id(
            identifier.clone(),
            Instant::now().add(active_cluster.timeouts().query_timeout()),
            ctrl_c.clone(),
        )?;
        let response = cloud.cloud_request(
            CloudRequest::GetUsers {
                cluster_id: cluster_id.clone(),
            },
            Instant::now().add(active_cluster.timeouts().query_timeout()),
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
                password.unwrap_or_default(),
                bucket_roles,
                all_access_roles,
            );
            cloud.cloud_request(
                CloudRequest::UpdateUser {
                    cluster_id,
                    payload: serde_json::to_string(&user)?,
                    username,
                },
                Instant::now().add(active_cluster.timeouts().query_timeout()),
                ctrl_c.clone(),
            )?
        } else {
            // Create
            let pass = match password {
                Some(p) => p,
                None => {
                    return Err(ShellError::unexpected(
                        "Cloud database user does not exist, password must be set",
                    ));
                }
            };

            let user =
                JSONCloudCreateUserRequest::new(username, pass, bucket_roles, all_access_roles);
            cloud.cloud_request(
                CloudRequest::CreateUser {
                    cluster_id,
                    payload: serde_json::to_string(&user)?,
                },
                Instant::now().add(active_cluster.timeouts().query_timeout()),
                ctrl_c.clone(),
            )?
        }
    } else {
        let form = &[
            ("name", display_name),
            ("groups", groups),
            ("roles", Some(roles)),
            ("password", password),
        ];
        let payload = serde_urlencoded::to_string(form).unwrap();

        active_cluster.cluster().http_client().management_request(
            ManagementRequest::UpsertUser { username, payload },
            Instant::now().add(active_cluster.timeouts().query_timeout()),
            ctrl_c,
        )?
    };

    match response.status() {
        200 => Ok(OutputStream::empty()),
        201 => Ok(OutputStream::empty()),
        202 => Ok(OutputStream::empty()),
        204 => Ok(OutputStream::empty()),
        _ => Err(ShellError::untagged_runtime_error(
            response.content().to_string(),
        )),
    }
}
