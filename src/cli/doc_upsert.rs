//! The `doc upsert` command performs a KV upsert operation.

use super::util::convert_nu_value_to_json_value;

use crate::state::State;
use couchbase::UpsertOptions;

use crate::cli::util::{collection_from_args, run_interruptable};
use async_trait::async_trait;
use futures::{FutureExt, StreamExt};
use nu_cli::{CommandArgs, OutputStream};
use nu_errors::ShellError;
use nu_protocol::{MaybeOwned, Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue};
use nu_source::Tag;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

pub struct DocUpsert {
    state: Arc<State>,
}

impl DocUpsert {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_cli::WholeStreamCommand for DocUpsert {
    fn name(&self) -> &str {
        "doc upsert"
    }

    fn signature(&self) -> Signature {
        Signature::build("doc upsert")
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
            .named("scope", SyntaxShape::String, "the name of the scope", None)
            .named(
                "collection",
                SyntaxShape::String,
                "the name of the collection",
                None,
            )
    }

    fn usage(&self) -> &str {
        "Upsert (insert or override) a document through the data service"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        run_upsert(self.state.clone(), args).await
    }
}

async fn run_upsert(state: Arc<State>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once().await?;
    let ctrl_c = args.ctrl_c.clone();

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

    let expiry = args
        .get("expiry")
        .map(|e| Duration::from_secs(e.as_u64().unwrap_or_else(|_| 0)));

    let collection = match collection_from_args(&args, &state) {
        Ok(c) => c,
        Err(e) => {
            return Err(e);
        }
    };

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
            let ctrl_c_clone = ctrl_c.clone();
            async move {
                let mut options = UpsertOptions::default();
                if let Some(e) = expiry {
                    options = options.expiry(e);
                }

                let upsert = collection.upsert(id, content, options);
                run_interruptable(upsert, ctrl_c_clone.clone()).await
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
