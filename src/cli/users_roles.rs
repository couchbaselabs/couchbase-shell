use crate::cli::util::{cluster_identifiers_from, validate_is_not_cloud};
use crate::state::State;
use async_trait::async_trait;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, TaggedDictBuilder};
use std::ops::Add;
use tokio::time::Instant;

use crate::cli::user_builder::RoleAndDescription;
use crate::client::ManagementRequest;
use nu_source::Tag;
use nu_stream::OutputStream;
use std::sync::{Arc, Mutex};

pub struct UsersRoles {
    state: Arc<Mutex<State>>,
}

impl UsersRoles {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for UsersRoles {
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
    }

    fn usage(&self) -> &str {
        "Shows all roles available on the cluster"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        run_async(self.state.clone(), args)
    }
}

fn run_async(state: Arc<Mutex<State>>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();

    let cluster_identifiers = cluster_identifiers_from(&state, &args, true)?;

    let permission = args.get_flag("permission")?;

    let mut entries = vec![];
    for identifier in cluster_identifiers {
        let guard = state.lock().unwrap();
        let active_cluster = match guard.clusters().get(&identifier) {
            Some(c) => c,
            None => {
                return Err(ShellError::unexpected("Cluster not found"));
            }
        };
        validate_is_not_cloud(
            active_cluster,
            "user roles cannot be run against cloud clusters",
        )?;

        let response = active_cluster.cluster().http_client().management_request(
            ManagementRequest::GetRoles {
                permission: permission.clone(),
            },
            Instant::now().add(active_cluster.timeouts().management_timeout()),
            ctrl_c.clone(),
        )?;

        let roles: Vec<RoleAndDescription> = match response.status() {
            200 => match serde_json::from_str(response.content()) {
                Ok(m) => m,
                Err(e) => {
                    return Err(ShellError::unexpected(format!(
                        "Failed to decode response body {}",
                        e,
                    )));
                }
            },
            _ => {
                return Err(ShellError::unexpected(format!(
                    "Request failed {}",
                    response.content(),
                )));
            }
        };

        for role_and_desc in roles {
            let mut collected = TaggedDictBuilder::new(Tag::default());

            collected.insert_value("cluster", identifier.clone());

            let role = role_and_desc.role();
            collected.insert_value("name", role_and_desc.display_name());
            collected.insert_value("role", role.name());
            collected.insert_value("bucket", role.bucket().unwrap_or_default());
            collected.insert_value("scope", role.scope().unwrap_or_default());
            collected.insert_value("collection", role.collection().unwrap_or_default());
            collected.insert_value("description", role_and_desc.description());

            entries.push(collected.into_value());
        }
    }

    Ok(entries.into())
}
