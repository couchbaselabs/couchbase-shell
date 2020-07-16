//! The `doc replace` command performs a KV replace operation.

use super::util::convert_nu_value_to_json_value;
use crate::state::State;
use couchbase::ReplaceOptions;

use async_trait::async_trait;
use futures::{FutureExt, StreamExt};
use nu_cli::{CommandArgs, CommandRegistry, OutputStream};
use nu_errors::ShellError;
use nu_protocol::{MaybeOwned, Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue};
use nu_source::Tag;
use std::sync::Arc;
use std::time::Duration;

pub struct DocReplace {
    state: Arc<State>,
}

impl DocReplace {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_cli::WholeStreamCommand for DocReplace {
    fn name(&self) -> &str {
        "doc replace"
    }

    fn signature(&self) -> Signature {
        Signature::build("doc replace")
            .optional("id", SyntaxShape::String, "the document id")
            .optional("content", SyntaxShape::String, "the document content")
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
            .named(
                "content-column",
                SyntaxShape::String,
                "the name of the content column if used with an input stream",
                None,
            )
            .named(
                "expiry",
                SyntaxShape::Number,
                "the expiry for the documents in seconds, or absolute",
                None,
            )
    }

    fn usage(&self) -> &str {
        "Replace a document through the data service"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        run_replace(self.state.clone(), args, registry).await
    }
}

async fn run_replace(
    state: Arc<State>,
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry).await?;

    let id_column = args
        .get("id-column")
        .map(|id| id.as_string().ok())
        .flatten()
        .unwrap_or_else(|| String::from("id"));

    let content_column = args
        .get("content-column")
        .map(|content| content.as_string().ok())
        .flatten()
        .unwrap_or_else(|| String::from("content"));

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

    let expiry = args
        .get("expiry")
        .map(|e| Duration::from_secs(e.as_u64().unwrap_or_else(|_| 0)));

    let bucket = state.active_cluster().bucket(&bucket_name);
    let collection = Arc::new(bucket.default_collection());

    let input_args = if let Some(id) = args.nth(0) {
        if let Some(content) = args.nth(1) {
            let id = id.as_string()?;
            let content = serde_json::from_str(&content.as_string()?)?;
            vec![(id, content)]
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    let filtered = args.input.filter_map(move |i| {
        let id_column = id_column.clone();
        let content_column = content_column.clone();
        async move {
            if let UntaggedValue::Row(dict) = i.value {
                let mut id = None;
                let mut content = None;
                if let MaybeOwned::Borrowed(d) = dict.get_data(id_column.as_ref()) {
                    id = d.as_string().ok();
                }
                if let MaybeOwned::Borrowed(d) = dict.get_data(content_column.as_ref()) {
                    content = convert_nu_value_to_json_value(d).ok();
                }
                if let Some(i) = id {
                    if let Some(c) = content {
                        return Some((i, c));
                    }
                }
            }
            None
        }
    });

    let mapped = filtered
        .chain(futures::stream::iter(input_args))
        .map(move |(id, content)| {
            let collection = collection.clone();
            async move {
                let mut options = ReplaceOptions::default();
                if let Some(e) = expiry {
                    options = options.expiry(e);
                }
                collection.replace(id, content, options).await
            }
        })
        .buffer_unordered(1000)
        .fold((0, 0), |(mut success, mut failed), res| async move {
            match res {
                Ok(_) => success += 1,
                Err(_) => failed += 1,
            };
            (success, failed)
        })
        .map(|(success, failed)| {
            let tag = Tag::default();
            let mut collected = TaggedDictBuilder::new(&tag);
            collected.insert_untagged("processed", UntaggedValue::int(success + failed));
            collected.insert_untagged("success", UntaggedValue::int(success));
            collected.insert_untagged("failed", UntaggedValue::int(failed));

            collected.into_value()
        })
        .into_stream();

    Ok(OutputStream::from_input(mapped))
}
