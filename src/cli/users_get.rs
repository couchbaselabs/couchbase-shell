use crate::cli::cloud_json::JSONCloudUser;
use crate::cli::user_builder::UserAndMetadata;
use crate::client::{CloudRequest, ManagementRequest};
use crate::state::State;
use async_trait::async_trait;
use log::debug;
use nu_cli::TaggedDictBuilder;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, Value};
use nu_source::Tag;
use nu_stream::OutputStream;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

pub struct UsersGet {
    state: Arc<Mutex<State>>,
}

impl UsersGet {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for UsersGet {
    fn name(&self) -> &str {
        "users get"
    }

    fn signature(&self) -> Signature {
        Signature::build("users get").required(
            "username",
            SyntaxShape::String,
            "the username of the user",
        )
    }

    fn usage(&self) -> &str {
        "Fetches a user"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        users_get(self.state.clone(), args)
    }
}

fn users_get(state: Arc<Mutex<State>>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();
    let args = args.evaluate_once()?;
    let username = args.nth(0).expect("need username").as_string()?;

    debug!("Running users get {}", username);

    let guard = state.lock().unwrap();
    let active_cluster = guard.active_cluster();

    let stream: Vec<Value> = if let Some(c) = active_cluster.cloud() {
        let identifier = guard.active();
        let cloud = guard.cloud_for_cluster(c)?.cloud();
        let cluster_id = cloud.find_cluster_id(
            identifier.clone(),
            Instant::now().add(active_cluster.timeouts().query_timeout()),
            ctrl_c.clone(),
        )?;
        let response = cloud.cloud_request(
            CloudRequest::GetUsers { cluster_id },
            Instant::now().add(active_cluster.timeouts().query_timeout()),
            ctrl_c.clone(),
        )?;
        if response.status() != 200 {
            return Err(ShellError::unexpected(response.content()));
        }

        let users: Vec<JSONCloudUser> = serde_json::from_str(response.content())?;

        users
            .into_iter()
            .filter(|user| user.username() == username.clone())
            .map(|user| {
                let mut roles: Vec<String> = Vec::new();
                for role in user.roles().iter() {
                    for name in role.names() {
                        roles.push(format!("{}[{}]", role.bucket().clone(), name));
                    }
                }

                let mut collected = TaggedDictBuilder::new(Tag::default());
                collected.insert_value("username", user.username());
                collected.insert_value("display name", "");
                collected.insert_value("groups", "");
                collected.insert_value("roles", roles.join(","));
                collected.insert_value("password_last_changed", "");
                collected.into_value()
            })
            .collect()
    } else {
        let response = active_cluster.cluster().http_client().management_request(
            ManagementRequest::GetUser { username },
            Instant::now().add(active_cluster.timeouts().query_timeout()),
            ctrl_c,
        )?;

        let user_and_meta: UserAndMetadata = match response.status() {
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

        let user = user_and_meta.user();
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
        if let Some(changed) = user_and_meta.password_changed() {
            collected.insert_value("password_last_changed", changed)
        }

        vec![collected.into_value()]
    };

    Ok(OutputStream::from(stream))
}
