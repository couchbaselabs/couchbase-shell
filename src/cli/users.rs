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
use std::ops::Add;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

pub struct Users {
    state: Arc<Mutex<State>>,
}

impl Users {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
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

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let ctrl_c = args.ctrl_c();
        users_get_all(self.state.clone(), ctrl_c)
    }
}

fn users_get_all(
    state: Arc<Mutex<State>>,
    ctrl_c: Arc<AtomicBool>,
) -> Result<OutputStream, ShellError> {
    debug!("Running users get all");
    let guard = state.lock().unwrap();
    let active_cluster = guard.active_cluster();
    let response = active_cluster.cluster().management_request(
        ManagementRequest::GetUsers,
        Instant::now().add(active_cluster.timeouts().query_timeout()),
        ctrl_c,
    )?;

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
                .iter()
                .map(|r| match r.bucket() {
                    Some(b) => format!("{}[{}]", r.name(), b),
                    None => r.name().to_string(),
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
