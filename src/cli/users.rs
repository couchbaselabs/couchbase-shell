use crate::cli::user_builder::UserAndMetadata;
use crate::client::ManagementRequest;
use crate::state::State;
use async_trait::async_trait;
use log::debug;
use nu_cli::TaggedDictBuilder;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, Value};
use nu_source::Tag;
use nu_stream::OutputStream;
use std::sync::Arc;

pub struct Users {
    state: Arc<State>,
}

impl Users {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for Users {
    fn name(&self) -> &str {
        "users"
    }

    fn signature(&self) -> Signature {
        Signature::build("users")
    }

    fn usage(&self) -> &str {
        "Lists all users"
    }

    fn run(&self, _args: CommandArgs) -> Result<OutputStream, ShellError> {
        users_get_all(self.state.clone())
    }
}

fn users_get_all(state: Arc<State>) -> Result<OutputStream, ShellError> {
    debug!("Running users get all");
    let active_cluster = state.active_cluster();
    let response = active_cluster
        .cluster()
        .management_request(ManagementRequest::GetUsers)?;

    let users: Vec<UserAndMetadata> = match response.status() {
        200 => match serde_json::from_str(response.content()) {
            Ok(m) => m,
            Err(e) => {
                return Err(ShellError::untagged_runtime_error(format!(
                    "Failed to decode response body {}",
                    e,
                )));
            }
        },
        _ => {
            return Err(ShellError::untagged_runtime_error(format!(
                "Request failed {}",
                response.content(),
            )));
        }
    };

    let stream: Vec<Value> = users
        .into_iter()
        .map(|v| {
            let user = v.user();
            let roles: Vec<String> = user
                .roles()
                .into_iter()
                .map(|r| match r.bucket() {
                    Some(b) => format!("{}[{}]", r.name(), b),
                    None => format!("{}", r.name()),
                })
                .collect();

            let mut collected = TaggedDictBuilder::new(Tag::default());
            collected.insert_value("username", user.username());
            collected.insert_value("display name", user.display_name().unwrap_or_default());
            if let Some(groups) = user.groups() {
                collected.insert_value("groups", groups.join(","))
            }
            collected.insert_value("roles", roles.join(","));
            if let Some(changed) = v.password_changed() {
                collected.insert_value("password_last_changed", changed)
            }
            collected.into_value()
        })
        .collect();
    Ok(OutputStream::from(stream))
}
