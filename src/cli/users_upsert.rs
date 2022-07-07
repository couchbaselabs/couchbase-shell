use crate::cli::cloud_json::{JSONCloudCreateUserRequest, JSONCloudUser, JSONCloudUserRoles};
use crate::cli::util::{cluster_identifiers_from, get_active_cluster};
use crate::client::{CapellaRequest, ManagementRequest};
use crate::state::{CapellaEnvironment, State};
use log::debug;
use std::collections::HashMap;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

use crate::cli::error::{
    cant_run_against_hosted_capella_error, deserialize_error, generic_error,
    unexpected_status_code_error,
};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape};

#[derive(Clone)]
pub struct UsersUpsert {
    state: Arc<Mutex<State>>,
}

impl UsersUpsert {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for UsersUpsert {
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
            .category(Category::Custom("couchbase".into()))
    }

    fn usage(&self) -> &str {
        "Upserts a user"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        users_upsert(self.state.clone(), engine_state, stack, call, input)
    }
}

fn users_upsert(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();

    let username: String = call.req(engine_state, stack, 0)?;
    let roles: String = call.req(engine_state, stack, 1)?;
    let password = call.get_flag(engine_state, stack, "password")?;
    let display_name = call.get_flag(engine_state, stack, "display_name")?;
    let groups = call.get_flag(engine_state, stack, "groups")?;

    debug!("Running users upsert for user {}", &username);

    let cluster_identifiers = cluster_identifiers_from(&engine_state, stack, &state, &call, true)?;
    let guard = state.lock().unwrap();

    for identifier in cluster_identifiers {
        let active_cluster = get_active_cluster(identifier.clone(), &guard, span.clone())?;

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
                return Err(generic_error(
                    "Users with cluster scoped permissions can only be assigned one role",
                    None,
                    span,
                ));
            };

            let cloud = guard.capella_org_for_cluster(plane)?.client();
            let deadline = Instant::now().add(active_cluster.timeouts().management_timeout());
            let cluster = cloud.find_cluster(identifier.clone(), deadline, ctrl_c.clone())?;

            if cluster.environment() == CapellaEnvironment::Hosted {
                return Err(cant_run_against_hosted_capella_error("users upsert", span));
            }

            let response = cloud.capella_request(
                CapellaRequest::GetUsers {
                    cluster_id: cluster.id(),
                },
                deadline,
                ctrl_c.clone(),
            )?;

            let users: Vec<JSONCloudUser> = match response.status() {
                200 => serde_json::from_str(response.content())
                    .map_err(|e| deserialize_error(e.to_string(), call.span()))?,
                _ => {
                    return Err(unexpected_status_code_error(
                        response.status(),
                        response.content(),
                        call.span(),
                    ));
                }
            };

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
                        cluster_id: cluster.id(),
                        payload: serde_json::to_string(&user)
                            .map_err(|e| deserialize_error(e.to_string(), span))?,
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
                        return Err(generic_error(
                            "Capella database user does not exist, password must be set",
                            "Use the --password flag to set a password for the user".to_string(),
                            span,
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
                        cluster_id: cluster.id(),
                        payload: serde_json::to_string(&user)
                            .map_err(|e| deserialize_error(e.to_string(), span))?,
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
                return Err(unexpected_status_code_error(
                    response.status(),
                    response.content(),
                    call.span(),
                ));
            }
        }
    }

    Ok(PipelineData::new(span))
}
