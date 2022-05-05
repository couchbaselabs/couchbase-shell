use crate::cli::user_builder::RoleAndDescription;
use crate::cli::util::{
    cluster_identifiers_from, cluster_not_found_error, generic_unspanned_error,
    map_serde_deserialize_error_to_shell_error, validate_is_not_cloud, NuValueMap,
};
use crate::client::ManagementRequest;
use crate::state::State;
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
            .category(Category::Custom("couchbase".into()))
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

    let cluster_identifiers = cluster_identifiers_from(&engine_state, stack, &state, &call, true)?;

    let permission = call.get_flag(engine_state, stack, "permission")?;

    let mut entries = vec![];
    for identifier in cluster_identifiers {
        let guard = state.lock().unwrap();
        let active_cluster = match guard.clusters().get(&identifier) {
            Some(c) => c,
            None => {
                return Err(cluster_not_found_error(identifier, call.span()));
            }
        };
        validate_is_not_cloud(
            active_cluster,
            "user roles cannot be run against capella clusters",
        )?;

        let response = active_cluster.cluster().http_client().management_request(
            ManagementRequest::GetRoles {
                permission: permission.clone(),
            },
            Instant::now().add(active_cluster.timeouts().management_timeout()),
            ctrl_c.clone(),
        )?;

        let roles: Vec<RoleAndDescription> = match response.status() {
            200 => serde_json::from_str(response.content())
                .map_err(map_serde_deserialize_error_to_shell_error)?,
            _ => {
                return Err(generic_unspanned_error(
                    "Failed to get roles",
                    format!("Failed to get roles {}", response.content()),
                ));
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
        span,
    }
    .into_pipeline_data())
}
