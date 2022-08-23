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
            .category(Category::Custom("couchbase".to_string()))
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

    let cluster_identifiers = cluster_identifiers_from(engine_state, stack, &state, call, true)?;
    let guard = state.lock().unwrap();

    let mut results = vec![];
    for identifier in cluster_identifiers {
        let active_cluster = get_active_cluster(identifier.clone(), &guard, span)?;
        validate_is_not_cloud(active_cluster, "users get", span)?;

        let response = active_cluster.cluster().http_client().management_request(
            ManagementRequest::GetUser {
                username: username.clone(),
            },
            Instant::now().add(active_cluster.timeouts().management_timeout()),
            ctrl_c.clone(),
        )?;

        let user_and_meta: UserAndMetadata = match response.status() {
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

        let mut stream: Vec<Value> = vec![collected.into_value(span)];

        results.append(&mut stream);
    }

    Ok(Value::List {
        vals: results,
        span,
    }
    .into_pipeline_data())
}
