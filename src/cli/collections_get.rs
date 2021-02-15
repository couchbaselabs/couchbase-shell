//! The `collections get` command fetches all of the collection names from the server.

use crate::state::State;
use couchbase::GetAllScopesOptions;

use crate::cli::util::bucket_name_from_args;
use async_trait::async_trait;
use log::debug;
use nu_cli::OutputStream;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue, Value};
use nu_source::Tag;
use num_bigint::BigInt;
use std::sync::Arc;

pub struct CollectionsGet {
    state: Arc<State>,
}

impl CollectionsGet {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for CollectionsGet {
    fn name(&self) -> &str {
        "collections get"
    }

    fn signature(&self) -> Signature {
        Signature::build("collections get")
            .named(
                "bucket",
                SyntaxShape::String,
                "the name of the bucket",
                None,
            )
            .named("scope", SyntaxShape::String, "the name of the scope", None)
    }

    fn usage(&self) -> &str {
        "Fetches collections through the HTTP API"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        collections_get(self.state.clone(), args).await
    }
}

async fn collections_get(state: Arc<State>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once().await?;

    let bucket = bucket_name_from_args(&args, state.active_cluster())?;
    let scope = args.get("scope").map(|c| c.as_string().ok()).flatten();

    debug!(
        "Running collections get for bucket {:?}, scope {:?}",
        &bucket, &scope
    );

    let mgr = state.active_cluster().bucket(bucket.as_str()).collections();
    let result = mgr.get_all_scopes(GetAllScopesOptions::default()).await;

    match result {
        Ok(res) => {
            let mut results: Vec<Value> = vec![];
            for scope_res in res {
                if let Some(scope_name) = &scope {
                    if scope_name != scope_res.name() {
                        continue;
                    }
                }
                let collections = scope_res.collections();
                if collections.len() == 0 {
                    let mut collected = TaggedDictBuilder::new(Tag::default());
                    collected.insert_value("scope", scope_res.name());
                    collected.insert_value("collection", "");
                    collected.insert_value("max_expiry", UntaggedValue::duration(0));
                    results.push(collected.into_value());
                    continue;
                }

                for collection in collections {
                    let mut collected = TaggedDictBuilder::new(Tag::default());
                    collected.insert_value("scope", scope_res.name());
                    collected.insert_value("collection", collection.name());
                    collected.insert_value(
                        "max_expiry",
                        UntaggedValue::duration(BigInt::from(collection.max_expiry().as_nanos())),
                    );
                    results.push(collected.into_value());
                }
            }
            Ok(OutputStream::from(results))
        }
        Err(e) => Err(ShellError::untagged_runtime_error(format!("{}", e))),
    }
}
