use crate::cli::cloud_json::JSONCloudUser;
use crate::cli::user_builder::UserAndMetadata;
use crate::cli::util::{
    cant_run_against_hosted_capella_error, cluster_identifiers_from, cluster_not_found_error,
    generic_labeled_error, map_serde_deserialize_error_to_shell_error, NuValueMap,
};
use crate::client::{CapellaRequest, ManagementRequest};
use crate::state::{CapellaEnvironment, State};
use log::debug;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct UsersGet {
    state: Arc<Mutex<State>>,
}

impl UsersGet {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for UsersGet {
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
            .category(Category::Custom("couchbase".into()))
    }

    fn usage(&self) -> &str {
        "Fetches a user"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        users_get(self.state.clone(), engine_state, stack, call, input)
    }
}

fn users_get(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();
    let username: String = call.req(engine_state, stack, 0)?;

    debug!("Running users get {}", username);

    let cluster_identifiers = cluster_identifiers_from(&engine_state, stack, &state, &call, true)?;
    let guard = state.lock().unwrap();

    let mut results = vec![];
    for identifier in cluster_identifiers {
        let active_cluster = match guard.clusters().get(&identifier) {
            Some(c) => c,
            None => {
                return Err(cluster_not_found_error(identifier));
            }
        };

        let mut stream: Vec<Value> = if let Some(plane) = active_cluster.capella_org() {
            let cloud = guard.capella_org_for_cluster(plane)?.client();
            let deadline = Instant::now().add(active_cluster.timeouts().management_timeout());
            let cluster =
                cloud.find_cluster(identifier.clone(), deadline.clone(), ctrl_c.clone())?;

            if cluster.environment() == CapellaEnvironment::Hosted {
                return Err(cant_run_against_hosted_capella_error());
            }

            let response = cloud.capella_request(
                CapellaRequest::GetUsers {
                    cluster_id: cluster.id(),
                },
                deadline,
                ctrl_c.clone(),
            )?;
            if response.status() != 200 {
                return Err(generic_labeled_error(
                    "Failed to get users",
                    format!("Failed to get users {}", response.content()),
                ));
            }

            let users: Vec<JSONCloudUser> = serde_json::from_str(response.content())
                .map_err(map_serde_deserialize_error_to_shell_error)?;

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

                    let mut collected = NuValueMap::default();
                    collected.add_string("username", user.username(), span);
                    collected.add_string("display name", "", span);
                    collected.add_string("groups", "", span);
                    collected.add_string("roles", roles.join(","), span);
                    collected.add_string("password_last_changed", "", span);
                    collected.add_string("cluster", identifier.clone(), span);
                    collected.into_value(span)
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
                200 => serde_json::from_str(response.content())
                    .map_err(map_serde_deserialize_error_to_shell_error)?,
                _ => {
                    return Err(generic_labeled_error(
                        "Failed to get user",
                        format!("Failed to get user {}", response.content()),
                    ));
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

            let mut collected = NuValueMap::default();
            collected.add_string("username", user.username(), span);
            collected.add_string(
                "display name",
                user.display_name().unwrap_or_default(),
                span,
            );
            if let Some(groups) = user.groups() {
                collected.add_string("groups", groups.join(","), span)
            }
            collected.add_string("roles", roles.join(","), span);
            if let Some(changed) = user_and_meta.password_changed() {
                collected.add_string("password_last_changed", changed, span)
            }
            collected.add_string("cluster", identifier.clone(), span);

            vec![collected.into_value(span)]
        };

        results.append(&mut stream);
    }

    Ok(Value::List {
        vals: results,
        span,
    }
    .into_pipeline_data())
}
