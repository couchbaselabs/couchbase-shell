use crate::state::State;
use couchbase::GetAllUsersOptions;
use futures::executor::block_on;
use log::debug;
use nu_cli::{CommandArgs, CommandRegistry, OutputStream, TaggedDictBuilder};
use nu_errors::ShellError;
use nu_protocol::{Signature, Value};
use nu_source::Tag;
use std::sync::Arc;

pub struct Users {
    state: Arc<State>,
}

impl Users {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

impl nu_cli::WholeStreamCommand for Users {
    fn name(&self) -> &str {
        "users"
    }

    fn signature(&self) -> Signature {
        Signature::build("users")
    }

    fn usage(&self) -> &str {
        "Lists all users"
    }

    fn run(
        &self,
        _args: CommandArgs,
        _registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        block_on(users_get_all(self.state.clone()))
    }
}

async fn users_get_all(state: Arc<State>) -> Result<OutputStream, ShellError> {
    debug!("Running users get all");
    let mgr = state.active_cluster().cluster().users();
    let result = mgr.get_all_users(GetAllUsersOptions::default()).await;

    match result {
        Ok(res) => {
            let stream: Vec<Value> = res
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
        Err(e) => Err(ShellError::untagged_runtime_error(format!("{}", e))),
    }
}
