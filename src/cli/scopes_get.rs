use crate::cli::collections_get::Manifest;
use crate::client::ManagementRequest;
use crate::state::State;
use async_trait::async_trait;
use log::debug;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, TaggedDictBuilder, Value};
use nu_source::Tag;
use nu_stream::OutputStream;
use std::ops::Add;
use std::sync::Arc;
use tokio::time::Instant;

pub struct ScopesGet {
    state: Arc<State>,
}

impl ScopesGet {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for ScopesGet {
    fn name(&self) -> &str {
        "scopes get"
    }

    fn signature(&self) -> Signature {
        Signature::build("scopes get").named(
            "bucket",
            SyntaxShape::String,
            "the name of the bucket",
            None,
        )
    }

    fn usage(&self) -> &str {
        "Fetches scopes through the HTTP API"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        scopes_get(self.state.clone(), args)
    }
}

fn scopes_get(state: Arc<State>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();
    let args = args.evaluate_once()?;

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
                return Err(ShellError::untagged_runtime_error(
                    "Could not auto-select a bucket - please use --bucket instead".to_string(),
                ));
            }
        },
    };

    debug!("Running scopes get for bucket {:?}", &bucket);

    let active_cluster = state.active_cluster();
    let response = active_cluster.cluster().management_request(
        ManagementRequest::GetScopes { bucket },
        Instant::now().add(active_cluster.timeouts().query_timeout()),
        ctrl_c,
    )?;

    let manifest: Manifest = match response.status() {
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

    let mut results: Vec<Value> = vec![];
    for scope in manifest.scopes {
        let mut collected = TaggedDictBuilder::new(Tag::default());
        collected.insert_value("scope", scope.name);
        results.push(collected.into_value());
    }
    Ok(OutputStream::from(results))
}
