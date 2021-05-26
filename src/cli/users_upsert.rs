use crate::client::ManagementRequest;
use crate::state::State;
use async_trait::async_trait;
use log::debug;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_stream::OutputStream;
use std::ops::Add;
use std::sync::Arc;
use tokio::time::Instant;

pub struct UsersUpsert {
    state: Arc<State>,
}

impl UsersUpsert {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for UsersUpsert {
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

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        users_upsert(self.state.clone(), args)
    }
}

fn users_upsert(state: Arc<State>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();
    let args = args.evaluate_once()?;
    let username = match args.call_info.args.get("username") {
        Some(v) => match v.as_string() {
            Ok(uname) => uname,
            Err(e) => return Err(e),
        },
        None => return Err(ShellError::unexpected("username is required")),
    };
    let roles = match args.call_info.args.get("roles") {
        Some(v) => match v.as_string() {
            Ok(roles) => roles,
            Err(e) => return Err(e),
        },
        None => return Err(ShellError::unexpected("roles is required")),
    };
    let password = match args.call_info.args.get("password") {
        Some(v) => match v.as_string() {
            Ok(pwd) => Some(pwd),
            Err(e) => return Err(e),
        },
        None => None,
    };
    let display_name = match args.call_info.args.get("display_name") {
        Some(v) => match v.as_string() {
            Ok(pwd) => Some(pwd),
            Err(e) => return Err(e),
        },
        None => None,
    };
    let groups = match args.call_info.args.get("groups") {
        Some(v) => match v.as_string() {
            Ok(pwd) => Some(pwd),
            Err(e) => return Err(e),
        },
        None => None,
    };

    debug!("Running users upsert for user {}", &username);

    let form = &[
        ("name", display_name),
        ("groups", groups),
        ("roles", Some(roles)),
        ("password", password),
    ];
    let payload = serde_urlencoded::to_string(form).unwrap();

    let active_cluster = state.active_cluster();

    let response = active_cluster.cluster().management_request(
        ManagementRequest::UpsertUser { username, payload },
        Instant::now().add(active_cluster.timeouts().query_timeout()),
        ctrl_c,
    )?;

    match response.status() {
        200 => Ok(OutputStream::empty()),
        202 => Ok(OutputStream::empty()),
        _ => Err(ShellError::untagged_runtime_error(
            response.content().to_string(),
        )),
    }
}
