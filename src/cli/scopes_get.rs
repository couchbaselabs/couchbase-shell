//! The `collections get` command fetches all of the collection names from the server.

use crate::state::State;
use couchbase::GetAllScopesOptions;

use crate::cli::util::bucket_name_from_args;
use async_trait::async_trait;
use log::debug;
use nu_cli::OutputStream;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, TaggedDictBuilder, Value};
use nu_source::Tag;
use std::sync::Arc;

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

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        scopes_get(self.state.clone(), args).await
    }
}

async fn scopes_get(state: Arc<State>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once().await?;

    let bucket = bucket_name_from_args(&args, state.active_cluster())?;

    debug!("Running scopes get for bucket {:?}", &bucket);

    let mgr = state.active_cluster().bucket(bucket.as_str()).collections();
    let result = mgr.get_all_scopes(GetAllScopesOptions::default()).await;

    match result {
        Ok(res) => {
            let mut results: Vec<Value> = vec![];
            for scope_res in res {
                let mut collected = TaggedDictBuilder::new(Tag::default());
                collected.insert_value("scope", scope_res.name());
                results.push(collected.into_value());
            }
            Ok(OutputStream::from(results))
        }
        Err(e) => Err(ShellError::untagged_runtime_error(format!("{}", e))),
    }
}
