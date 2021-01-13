use crate::state::State;
use async_trait::async_trait;
use couchbase::GetUserOptions;
use log::debug;
use nu_cli::{CommandArgs, OutputStream, TaggedDictBuilder};
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_source::Tag;
use std::sync::Arc;

pub struct UsersGet {
    state: Arc<State>,
}

impl UsersGet {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_cli::WholeStreamCommand for UsersGet {
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

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        users_get(self.state.clone(), args).await
    }
}

async fn users_get(state: Arc<State>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once().await?;
    let username = args.nth(0).expect("need username").as_string()?;

    debug!("Running users get {}", username);

    let mgr = state.active_cluster().cluster().users();
    let result = mgr.get_user(username, GetUserOptions::default()).await;

    match result {
        Ok(res) => {
            let user = res.user();
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
            if let Some(changed) = res.password_changed() {
                collected.insert_value("password_last_changed", changed)
            }

            Ok(OutputStream::from(vec![collected.into_value()]))
        }
        Err(e) => Err(ShellError::untagged_runtime_error(format!("{}", e))),
    }
}
