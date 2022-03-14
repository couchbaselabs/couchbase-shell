use crate::cli::cloud_json::JSONCloudUser;
use crate::cli::user_builder::UserAndMetadata;
use crate::cli::util::{cluster_identifiers_from, NuValueMap};
use crate::client::{CapellaRequest, ManagementRequest};
use crate::state::{CapellaEnvironment, State};
use log::debug;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct Users {
    state: Arc<Mutex<State>>,
}

impl Users {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for Users {
    fn name(&self) -> &str {
        "users"
    }

    fn signature(&self) -> Signature {
        Signature::build("users")
            .named(
                "clusters",
                SyntaxShape::String,
                "the clusters which should be contacted",
                None,
            )
            .category(Category::Custom("couchbase".into()))
    }

    fn usage(&self) -> &str {
        "Lists all users"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        users_get_all(self.state.clone(), engine_state, stack, call, input)
    }
}

fn users_get_all(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();
    debug!("Running users get all");

    let cluster_identifiers = cluster_identifiers_from(engine_state, stack, &state, call, true)?;
    let guard = state.lock().unwrap();

    let mut results = vec![];
    for identifier in cluster_identifiers {
        let active_cluster = match guard.clusters().get(&identifier) {
            Some(c) => c,
            None => {
                return Err(ShellError::LabeledError(
                    "Cluster not found".into(),
                    "Cluster not found".into(),
                ));
            }
        };
        let mut stream: Vec<Value> = if let Some(plane) = active_cluster.capella_org() {
            let cloud = guard.capella_org_for_cluster(plane)?.client();
            let deadline = Instant::now().add(active_cluster.timeouts().management_timeout());
            let cluster =
                cloud.find_cluster(identifier.clone(), deadline.clone(), ctrl_c.clone())?;

            if cluster.environment() == CapellaEnvironment::Hosted {
                return Err(ShellError::IncompatibleParametersSingle(
                    "users cannot be run against hosted Capella clusters".into(),
                    call.head,
                ));
            }

            let response = cloud.capella_request(
                CapellaRequest::GetUsers {
                    cluster_id: cluster.id(),
                },
                deadline,
                ctrl_c.clone(),
            )?;
            if response.status() != 200 {
                return Err(ShellError::LabeledError(
                    response.content().to_string(),
                    response.content().to_string(),
                ));
            }

            let users: Vec<JSONCloudUser> = serde_json::from_str(response.content())
                .map_err(|e| ShellError::LabeledError(e.to_string(), e.to_string()))?;

            users
                .into_iter()
                .map(|user| {
                    let mut roles: Vec<String> = Vec::new();
                    for role in user.roles().iter() {
                        for name in role.names() {
                            roles.push(format!("{}[{}]", role.bucket().clone(), name));
                        }
                    }

                    let mut collected = NuValueMap::default();
                    collected.add_string("username", user.username(), call.head);
                    collected.add_string("display name", "", call.head);
                    collected.add_string("groups", "", call.head);
                    collected.add_string("roles", roles.join(","), call.head);
                    collected.add_string("password_last_changed", "", call.head);
                    collected.add_string("cluster", identifier.clone(), call.head);
                    collected.into_value(call.head)
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
                        return Err(ShellError::LabeledError(
                            format!("Failed to decode response body {}", e,),
                            "".into(),
                        ));
                    }
                },
                _ => {
                    return Err(ShellError::LabeledError(
                        format!("Request failed {}", response.content(),),
                        "".into(),
                    ));
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

                    let mut collected = NuValueMap::default();
                    collected.add_string("username", user.username(), call.head);
                    collected.add_string(
                        "display name",
                        user.display_name().unwrap_or_default(),
                        call.head,
                    );
                    if let Some(groups) = user.groups() {
                        collected.add_string("groups", groups.join(","), call.head)
                    } else {
                        collected.add_string("groups", "", call.head)
                    }
                    collected.add_string("roles", roles.join(","), call.head);
                    if let Some(changed) = v.password_changed() {
                        collected.add_string("password_last_changed", changed, call.head)
                    } else {
                        collected.add_string("password_last_changed", "", call.head)
                    }
                    collected.add_string("cluster", identifier.clone(), call.head);
                    collected.into_value(call.head)
                })
                .collect()
        };

        results.append(&mut stream);
    }

    Ok(Value::List {
        vals: results,
        span: call.head,
    }
    .into_pipeline_data())
}
