use crate::cli::error::{
    client_error_to_shell_error, deserialize_error, unexpected_status_code_error,
};
use crate::cli::user_builder::RoleAndDescription;
use crate::cli::util::{
    cluster_identifiers_from, get_active_cluster, validate_is_not_cloud, NuValueMap,
};
use crate::client::ManagementRequest;
use crate::state::State;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

#[derive(Clone)]
pub struct UsersRoles {
    state: Arc<Mutex<State>>,
}

impl UsersRoles {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for UsersRoles {
    fn name(&self) -> &str {
        "users roles"
    }

    fn signature(&self) -> Signature {
        Signature::build("users roles")
            .named(
                "clusters",
                SyntaxShape::String,
                "the clusters which should be contacted",
                None,
            )
            .named(
                "permission",
                SyntaxShape::String,
                "filter roles based on the permission string",
                None,
            )
            .category(Category::Custom("couchbase".to_string()))
    }

    fn usage(&self) -> &str {
        "Shows all roles available on the cluster"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        run_async(self.state.clone(), engine_state, stack, call, input)
    }
}

fn run_async(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();

    let cluster_identifiers = cluster_identifiers_from(engine_state, stack, &state, call, true)?;
    let guard = state.lock().unwrap();

    let permission = call.get_flag(engine_state, stack, "permission")?;

    let mut entries = vec![];
    for identifier in cluster_identifiers {
        let active_cluster = get_active_cluster(identifier.clone(), &guard, span)?;
        validate_is_not_cloud(active_cluster, "user roles", span)?;

        let response = active_cluster
            .cluster()
            .http_client()
            .management_request(
                ManagementRequest::GetRoles {
                    permission: permission.clone(),
                },
                Instant::now().add(active_cluster.timeouts().management_timeout()),
                ctrl_c.clone(),
            )
            .map_err(|e| client_error_to_shell_error(e, span))?;

        let status = response.status();
        let content = response.content()?;
        let roles: Vec<RoleAndDescription> = match status {
            200 => serde_json::from_str(&content)
                .map_err(|e| deserialize_error(e.to_string(), span))?,
            _ => {
                return Err(unexpected_status_code_error(status, content, call.span()));
            }
        };

        for role_and_desc in roles {
            let mut collected = NuValueMap::default();

            collected.add_string("cluster", identifier.clone(), span);

            let role = role_and_desc.role();
            collected.add_string("name", role_and_desc.display_name(), span);
            collected.add_string("role", role.name(), span);
            collected.add_string("bucket", role.bucket().unwrap_or_default(), span);
            collected.add_string("scope", role.scope().unwrap_or_default(), span);
            collected.add_string("collection", role.collection().unwrap_or_default(), span);
            collected.add_string("description", role_and_desc.description(), span);

            entries.push(collected.into_value(span));
        }
    }

    Ok(Value::List {
        vals: entries,
        internal_span: span,
    }
    .into_pipeline_data())
}
