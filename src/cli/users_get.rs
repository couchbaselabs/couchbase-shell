use crate::cli::user_builder::UserAndMetadata;
use crate::client::ManagementRequest;
use crate::state::State;
use async_trait::async_trait;
use log::debug;
use nu_cli::TaggedDictBuilder;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_source::Tag;
use nu_stream::OutputStream;
use std::ops::Add;
use std::sync::Arc;
use tokio::time::Instant;

pub struct UsersGet {
    state: Arc<State>,
}

impl UsersGet {
    pub fn new(state: Arc<State>) -> Self {
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

fn users_get(state: Arc<State>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once()?;
    let username = args.nth(0).expect("need username").as_string()?;

    debug!("Running users get {}", username);

    let active_cluster = state.active_cluster();
    let response = active_cluster.cluster().management_request(
        ManagementRequest::GetUser { username },
        Instant::now().add(active_cluster.timeouts().query_timeout()),
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
    if let Some(changed) = user_and_meta.password_changed() {
        collected.insert_value("password_last_changed", changed)
    }

    Ok(OutputStream::from(vec![collected.into_value()]))
}
