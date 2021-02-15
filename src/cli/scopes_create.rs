//! The `collections get` command fetches all of the collection names from the server.

use crate::state::State;
use couchbase::CreateScopeOptions;

use crate::cli::util::bucket_name_from_args;
use async_trait::async_trait;
use log::debug;
use nu_cli::OutputStream;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use std::sync::Arc;

pub struct ScopesCreate {
    state: Arc<State>,
}

impl ScopesCreate {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for ScopesCreate {
    fn name(&self) -> &str {
        "scopes create"
    }

    fn signature(&self) -> Signature {
        Signature::build("scopes create")
            .required_named("name", SyntaxShape::String, "the name of the scope", None)
            .named(
                "bucket",
                SyntaxShape::String,
                "the name of the bucket",
                None,
            )
    }

    fn usage(&self) -> &str {
        "Creates scopes through the HTTP API"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        scopes_create(self.state.clone(), args).await
    }
}

async fn scopes_create(state: Arc<State>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once().await?;

    let scope = match args.get("name") {
        Some(v) => match v.as_string() {
            Ok(uname) => uname,
            Err(e) => return Err(e),
        },
        None => return Err(ShellError::unexpected("name is required")),
    };

    let bucket = bucket_name_from_args(&args, state.active_cluster())?;

    debug!(
        "Running scope create for {:?} on bucket {:?}",
        &scope, &bucket
    );

    let mgr = state.active_cluster().bucket(bucket.as_str()).collections();
    let result = mgr.create_scope(scope, CreateScopeOptions::default()).await;

    match result {
        Ok(_) => Ok(OutputStream::empty()),
        Err(e) => Err(ShellError::untagged_runtime_error(format!("{}", e))),
    }
}
