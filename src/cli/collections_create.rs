//! The `collections get` command fetches all of the collection names from the server.

use crate::state::State;

use crate::client::ManagementRequest;
use crate::client::ManagementRequest::{CreateBucket, CreateCollection};
use async_trait::async_trait;
use log::debug;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_source::Tag;
use nu_stream::OutputStream;
use std::sync::Arc;
use std::time::Duration;

pub struct CollectionsCreate {
    state: Arc<State>,
}

impl CollectionsCreate {
    pub fn new(state: Arc<State>) -> Self {
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

fn collections_create(state: Arc<State>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once()?;

    let active_cluster = state.active_cluster();
    let collection = match args.call_info.args.get("name") {
        Some(v) => match v.as_string() {
            Ok(uname) => uname,
            Err(e) => return Err(e),
        },
        None => return Err(ShellError::unexpected("name is required")),
    };

    let bucket = match args
        .call_info
        .args
        .get("bucket")
        .map(|bucket| bucket.as_string().ok())
        .flatten()
    {
        Some(v) => v,
        None => match state.active_cluster().active_bucket() {
            Some(s) => s,
            None => {
                return Err(ShellError::untagged_runtime_error(format!(
                    "Could not auto-select a bucket - please use --bucket instead"
                )));
            }
        },
    };

    let scope_name = match args
        .call_info
        .args
        .get("scope")
        .map(|c| c.as_string().ok())
        .flatten()
    {
        Some(name) => name,
        None => match state.active_cluster().active_scope() {
            Some(s) => s,
            None => {
                return Err(ShellError::untagged_runtime_error(format!(
                    "Could not auto-select a scope - please use --scope instead"
                )));
            }
        },
    };
    let expiry = match args.call_info.args.get("max-expiry") {
        Some(v) => match v.as_u64() {
            Ok(e) => e,
            Err(e) => return Err(e),
        },
        None => 0,
    };

    debug!(
        "Running collections create for {:?} on bucket {:?}, scope {:?}",
        &collection, &bucket, &scope_name
    );

    let mut form = vec![("name", collection)];
    if expiry > 0 {
        form.push(("maxTTL", expiry.to_string()));
    }

    let form_encoded = serde_urlencoded::to_string(&form).unwrap();

    let response =
        active_cluster
            .cluster()
            .management_request(ManagementRequest::CreateCollection {
                scope: scope_name,
                bucket,
                payload: form_encoded,
            })?;

    match response.status() {
        200 => Ok(OutputStream::empty()),
        202 => Ok(OutputStream::empty()),
        _ => Err(ShellError::untagged_runtime_error(format!(
            "{}",
            response.content()
        ))),
    }
}
