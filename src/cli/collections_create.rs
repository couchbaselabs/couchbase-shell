//! The `collections get` command fetches all of the collection names from the server.

use crate::client::ManagementRequest::CreateCollection;
use crate::state::State;
use async_trait::async_trait;
use log::debug;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_stream::OutputStream;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

pub struct CollectionsCreate {
    state: Arc<Mutex<State>>,
}

impl CollectionsCreate {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for CollectionsCreate {
    fn name(&self) -> &str {
        "collections create"
    }

    fn signature(&self) -> Signature {
        Signature::build("collections create")
            .required_named(
                "name",
                SyntaxShape::String,
                "the name of the collection",
                None,
            )
            .named(
                "bucket",
                SyntaxShape::String,
                "the name of the bucket",
                None,
            )
            .named("scope", SyntaxShape::String, "the name of the scope", None)
            .named(
                "max-expiry",
                SyntaxShape::Int,
                "the maximum expiry for documents in this collection, in seconds",
                None,
            )
    }

    fn usage(&self) -> &str {
        "Creates collections through the HTTP API"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        collections_create(self.state.clone(), args)
    }
}

fn collections_create(
    state: Arc<Mutex<State>>,
    args: CommandArgs,
) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();

    let guard = state.lock().unwrap();
    let active_cluster = guard.active_cluster();
    let collection: String = args.req_named("name")?;

    let bucket = match args.get_flag("bucket")? {
        Some(v) => v,
        None => match active_cluster.active_bucket() {
            Some(s) => s,
            None => {
                return Err(ShellError::untagged_runtime_error(
                    "Could not auto-select a bucket - please use --bucket instead".to_string(),
                ));
            }
        },
    };

    let scope_name = match args.get_flag("scope")? {
        Some(name) => name,
        None => match active_cluster.active_scope() {
            Some(s) => s,
            None => {
                return Err(ShellError::untagged_runtime_error(
                    "Could not auto-select a scope - please use --scope instead".to_string(),
                ));
            }
        },
    };
    let expiry = args.get_flag("max-expiry")?.unwrap_or(0);

    debug!(
        "Running collections create for {:?} on bucket {:?}, scope {:?}",
        &collection, &bucket, &scope_name
    );

    let mut form = vec![("name", collection)];
    if expiry > 0 {
        form.push(("maxTTL", expiry.to_string()));
    }

    let form_encoded = serde_urlencoded::to_string(&form).unwrap();

    let response = active_cluster.cluster().http_client().management_request(
        CreateCollection {
            scope: scope_name,
            bucket,
            payload: form_encoded,
        },
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
