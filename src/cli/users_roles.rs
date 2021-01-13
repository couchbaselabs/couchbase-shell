use crate::cli::convert_cb_error;
use crate::cli::util::cluster_identifiers_from;
use crate::state::State;
use async_trait::async_trait;
use couchbase::{GenericManagementRequest, Request};
use futures::channel::oneshot;
use nu_cli::{CommandArgs, OutputStream};
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue};

use nu_source::Tag;
use serde::Deserialize;
use std::sync::Arc;

pub struct UsersRoles {
    state: Arc<State>,
}

impl UsersRoles {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_cli::WholeStreamCommand for UsersRoles {
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

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        run_async(self.state.clone(), args).await
    }
}

async fn run_async(state: Arc<State>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once().await?;

    let cluster_identifiers = cluster_identifiers_from(&state, &args, true)?;

    let permission = args
        .get("permission")
        .map(|id| id.as_string().ok())
        .flatten();

    let mut entries = vec![];
    for identifier in cluster_identifiers {
        let core = match state.clusters().get(&identifier) {
            Some(c) => c.cluster().core(),
            None => {
                return Err(ShellError::untagged_runtime_error("Cluster not found"));
            }
        };

        let (sender, receiver) = oneshot::channel();

        let path = if let Some(ref p) = permission {
            format!("/settings/rbac/roles?permission={}", p)
        } else {
            "/settings/rbac/roles".into()
        };

        let request = GenericManagementRequest::new(sender, path, "get".into(), None);
        core.send(Request::GenericManagementRequest(request));

        let input = match receiver.await {
            Ok(i) => i,
            Err(e) => {
                return Err(ShellError::untagged_runtime_error(format!(
                    "Error streaming result {}",
                    e
                )))
            }
        };
        let result = convert_cb_error(input)?;

        if result.payload().is_none() {
            return Err(ShellError::untagged_runtime_error(
                "Empty response from cluster even though got 200 ok",
            ));
        }

        let payload = match result.payload() {
            Some(p) => p,
            None => {
                return Err(ShellError::untagged_runtime_error(
                    "Empty response from cluster even though got 200 ok",
                ));
            }
        };

        let data = serde_json::from_slice::<Vec<Role>>(payload)?;

        for role in data {
            let mut collected = TaggedDictBuilder::new(Tag::default());

            collected.insert_value("cluster", identifier.clone());

            collected.insert_value("name", role.name);
            collected.insert_value("role", role.role);
            collected.insert_value("ce", UntaggedValue::boolean(role.ce.unwrap_or_default()));
            collected.insert_value("bucket", role.bucket_name.unwrap_or_default());
            collected.insert_value("description", role.desc);

            entries.push(collected.into_value());
        }
    }

    Ok(entries.into())
}

#[derive(Debug, Deserialize)]
struct Role {
    name: String,
    role: String,
    desc: String,
    ce: Option<bool>,
    bucket_name: Option<String>,
}
