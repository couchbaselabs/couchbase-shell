use crate::state::State;
use async_trait::async_trait;
use couchbase::{Role, UpsertUserOptions, UserBuilder};
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

#[async_trait]
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

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        users_upsert(self.state.clone(), args, registry).await
    }
}

async fn users_upsert(
    state: Arc<State>,
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry).await?;
    let username = match args.get("username") {
        Some(v) => match v.as_string() {
            Ok(uname) => uname,
            Err(e) => return Err(e),
        },
        None => return Err(ShellError::unexpected("username is required")),
    };
    let roles_string = match args.get("roles") {
        Some(v) => match v.as_string() {
            Ok(roles) => roles,
            Err(e) => return Err(e),
        },
        None => return Err(ShellError::unexpected("username is required")),
    };
    let password = match args.get("password") {
        Some(v) => match v.as_string() {
            Ok(pwd) => Some(pwd),
            Err(e) => return Err(e),
        },
        None => None,
    };
    let display_name = match args.get("display_name") {
        Some(v) => match v.as_string() {
            Ok(pwd) => Some(pwd),
            Err(e) => return Err(e),
        },
        None => None,
    };
    let groups = match args.get("groups") {
        Some(v) => match v.as_string() {
            Ok(pwd) => Some(pwd),
            Err(e) => return Err(e),
        },
        None => None,
    };

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
