use crate::cli::user_builder::UserAndMetadata;
use crate::cli::util::{
    cluster_identifiers_from, get_active_cluster, validate_is_not_cloud, NuValueMap,
};
use crate::client::ManagementRequest;
use crate::state::State;
use log::debug;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

use crate::cli::error::{deserialize_error, unexpected_status_code_error};
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
            .category(Category::Custom("couchbase".to_string()))
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
    let span = call.head;
    let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();
    debug!("Running users get all");

    let cluster_identifiers = cluster_identifiers_from(engine_state, stack, &state, call, true)?;
    let guard = state.lock().unwrap();

    let mut results = vec![];
    for identifier in cluster_identifiers {
        let active_cluster = get_active_cluster(identifier.clone(), &guard, span.clone())?;
        validate_is_not_cloud(active_cluster, "users", span)?;

        let response = active_cluster.cluster().http_client().management_request(
            ManagementRequest::GetUsers,
            Instant::now().add(active_cluster.timeouts().management_timeout()),
            ctrl_c.clone(),
        )?;

        let users: Vec<UserAndMetadata> = match response.status() {
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

        let mut stream: Vec<Value> = users
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
            .collect();

        results.append(&mut stream);
    }

    Ok(Value::List {
        vals: results,
        span,
    }
    .into_pipeline_data())
}
