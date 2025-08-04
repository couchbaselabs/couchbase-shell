use crate::cli::util::{cluster_identifiers_from, get_active_cluster, validate_is_not_cloud};
use crate::client::ManagementRequest;
use crate::state::State;
use log::debug;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

use crate::cli::error::{client_error_to_shell_error, unexpected_status_code_error};
use nu_engine::command_prelude::Call;
use nu_engine::CallExt;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::Value::Nothing;
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
            .category(Category::Custom("couchbase".to_string()))
    }

    fn description(&self) -> &str {
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
    let signals = engine_state.signals().clone();

    let username: String = call.req(engine_state, stack, 0)?;
    let roles: String = call.req(engine_state, stack, 1)?;
    let password = call.get_flag(engine_state, stack, "password")?;
    let display_name = call.get_flag(engine_state, stack, "display_name")?;
    let groups = call.get_flag(engine_state, stack, "groups")?;

    debug!("Running users upsert for user {}", &username);

    let cluster_identifiers = cluster_identifiers_from(engine_state, stack, &state, call, true)?;
    let guard = state.lock().unwrap();

    for identifier in cluster_identifiers {
        let active_cluster = get_active_cluster(identifier.clone(), &guard, span)?;
        validate_is_not_cloud(active_cluster, "users upsert", span)?;

        let form = &[
            ("name", display_name.clone()),
            ("groups", groups.clone()),
            ("roles", Some(roles.clone())),
            ("password", password.clone()),
        ];
        let payload = serde_urlencoded::to_string(form).unwrap();

        let response = active_cluster
            .cluster()
            .http_client()
            .management_request(
                ManagementRequest::UpsertUser {
                    username: username.clone(),
                    payload,
                },
                Instant::now().add(active_cluster.timeouts().management_timeout()),
                signals.clone(),
            )
            .map_err(|e| client_error_to_shell_error(e, span))?;

        match response.status() {
            200 => {}
            201 => {}
            202 => {}
            204 => {}
            _ => {
                return Err(unexpected_status_code_error(
                    response.status(),
                    response.content()?,
                    call.span(),
                ));
            }
        }
    }

    Ok(PipelineData::Value(
        Nothing {
            internal_span: span,
        },
        None,
    ))
}
