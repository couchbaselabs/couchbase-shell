//! The `doc remove` command performs a KV remove operation.

use crate::state::State;
use couchbase::RemoveOptions;

use crate::cli::util::run_interruptable;
use async_trait::async_trait;
use futures::stream::StreamExt;
use futures::FutureExt;
use nu_cli::{CommandArgs, CommandRegistry, OutputStream};
use nu_errors::ShellError;
use nu_protocol::{MaybeOwned, Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue};
use nu_source::Tag;
use std::collections::HashSet;
use std::sync::Arc;

pub struct DocRemove {
    state: Arc<State>,
}

impl DocRemove {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_cli::WholeStreamCommand for DocRemove {
    fn name(&self) -> &str {
        "doc remove"
    }

    fn signature(&self) -> Signature {
        Signature::build("doc remove")
            .optional("id", SyntaxShape::String, "the document id")
            .named(
                "id-column",
                SyntaxShape::String,
                "the name of the id column if used with an input stream",
                None,
            )
            .named(
                "bucket",
                SyntaxShape::String,
                "the name of the bucket",
                None,
            )
    }

    fn usage(&self) -> &str {
        "Removes a document through the data service"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        run_get(self.state.clone(), args, registry).await
    }
}

async fn run_get(
    state: Arc<State>,
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry).await?;
    let ctrl_c = args.ctrl_c.clone();

    let id_column = args
        .get("id-column")
        .map(|id| id.as_string().ok())
        .flatten()
        .unwrap_or_else(|| String::from("id"));

    let bucket_name = match args
        .get("bucket")
        .map(|bucket| bucket.as_string().ok())
        .flatten()
        .or_else(|| state.active_cluster().active_bucket())
    {
        Some(v) => v,
        None => {
            return Err(ShellError::untagged_runtime_error(format!(
                "Could not auto-select a bucket - please use --bucket instead"
            )))
        }
    };

    let bucket = state.active_cluster().bucket(&bucket_name);
    let collection = Arc::new(bucket.default_collection());

    let input_args = if let Some(id) = args.nth(0) {
        vec![id.as_string()?]
    } else {
        vec![]
    };

    let filtered = args.input.filter_map(move |i| {
        let id_column = id_column.clone();
        async move {
            if let UntaggedValue::Row(dict) = i.value {
                if let MaybeOwned::Borrowed(d) = dict.get_data(id_column.as_ref()) {
                    return d.as_string().ok();
                }
            }
            None
        }
    });

    let mapped = filtered
        .chain(futures::stream::iter(input_args))
        .map(move |id| {
            let collection = collection.clone();
            let ctrl_c_clone = ctrl_c.clone();
            async move {
                let remove = collection.remove(id, RemoveOptions::default());
                run_interruptable(remove, ctrl_c_clone.clone()).await
            }
        })
        .buffer_unordered(1000)
        .fold(
            (0, 0, HashSet::new()),
            |(mut success, mut failed, mut fail_reasons), res| async move {
                match res {
                    Ok(_) => success += 1,
                    Err(e) => {
                        fail_reasons.insert(e.to_string());
                        failed += 1;
                    }
                };
                (success, failed, fail_reasons)
            },
        )
        .map(|(success, failed, fail_reasons)| {
            let tag = Tag::default();
            let mut collected = TaggedDictBuilder::new(&tag);
            collected.insert_untagged("processed", UntaggedValue::int(success + failed));
            collected.insert_untagged("success", UntaggedValue::int(success));
            collected.insert_untagged("failed", UntaggedValue::int(failed));

            let reasons = fail_reasons
                .iter()
                .map(|v| {
                    let mut collected_fails = TaggedDictBuilder::new(&tag);
                    collected_fails.insert_untagged("fail reason", UntaggedValue::string(v));
                    collected_fails.into()
                })
                .collect();
            collected.insert_untagged("failures", UntaggedValue::Table(reasons));

            collected.into_value()
        })
        .into_stream();

    Ok(OutputStream::from_input(mapped))
}
