use crate::state::State;
use couchbase::{Role, UpsertUserOptions, UserBuilder};
use futures::executor::block_on;
use log::debug;
use nu_cli::{CommandArgs, CommandRegistry, OutputStream};
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use std::sync::Arc;

pub struct UsersUpsert {
    state: Arc<State>,
}

impl UsersUpsert {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

impl nu_cli::WholeStreamCommand for UsersUpsert {
    fn name(&self) -> &str {
        "users upsert"
    }

    fn signature(&self) -> Signature {
        Signature::build("users upsert")
            .required_named(
                "username",
                SyntaxShape::String,
                "the username of the user",
                None,
            )
            .required_named(
                "roles",
                SyntaxShape::String,
                "the roles for the user <role_name[bucket_name]>",
                None,
            )
            .named(
                "password",
                SyntaxShape::String,
                "the password for the user",
                None,
            )
            .named(
                "display_name",
                SyntaxShape::String,
                "the display name for the user",
                None,
            )
            .named(
                "groups",
                SyntaxShape::String,
                "the group names for the user",
                None,
            )
    }

    fn usage(&self) -> &str {
        "Upserts a user"
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        block_on(users_upsert(self.state.clone(), args, registry))
    }
}

async fn users_upsert(
    state: Arc<State>,
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry).await?;
    let username = args.get("username").expect("need username").as_string()?;
    let roles_string = args.get("roles").expect("need roles").as_string()?;
    let password = args
        .get("password")
        .map(|password| password.as_string().unwrap());
    let display_name = args.get("display_name").map(|dn| dn.as_string().unwrap());
    let groups = args.get("groups").map(|g| g.as_string().unwrap());

    let roles = roles_string
        .split(",")
        .collect::<Vec<&str>>()
        .iter()
        .map(|role| {
            let role_sp = roles_string.split("[").collect::<Vec<&str>>();
            if role_sp.len() > 1 {
                Role::new(
                    role_sp[0].to_string(),
                    Some(role_sp[1].to_string().replace("]", "")),
                )
            } else {
                Role::new(role.to_string(), None)
            }
        })
        .collect();

    debug!("Running users upsert for user {}", &username);

    let mgr = state.active_cluster().cluster().users();
    let mut builder = UserBuilder::new(username, password, roles);
    if let Some(dname) = display_name {
        builder = builder.display_name(dname);
    }
    if let Some(g) = groups {
        builder = builder.display_name(g);
    }

    let result = mgr
        .upsert_user(builder.build(), UpsertUserOptions::default())
        .await;

    match result {
        Ok(_) => Ok(OutputStream::empty()),
        Err(e) => Err(ShellError::untagged_runtime_error(format!("{}", e))),
    }
}
