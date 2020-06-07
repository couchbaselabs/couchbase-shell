//! The `kv remove` command performs a KV remove operation.

use crate::state::State;
use couchbase::RemoveOptions;

use async_trait::async_trait;
use futures::stream::StreamExt;
use futures::FutureExt;
use nu_cli::{CommandArgs, CommandRegistry, OutputStream};
use nu_errors::ShellError;
use nu_protocol::{MaybeOwned, Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue};
use nu_source::Tag;
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

    let id_column = args
        .get("id-column")
        .map(|id| id.as_string().unwrap())
        .unwrap_or_else(|| String::from("id"));

    let bucket_name = match args
        .get("bucket")
        .map(|id| id.as_string().unwrap())
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

    let input_args = if args.nth(0).is_some() {
        let id = args.nth(0).unwrap().as_string()?;
        vec![id]
    } else {
        vec![]
    };

    let filtered = args.input.filter_map(move |i| {
        let id_column = id_column.clone();
        async move {
            if let UntaggedValue::Row(dict) = i.value {
                let mut id = None;
                if let MaybeOwned::Borrowed(d) = dict.get_data(id_column.as_ref()) {
                    id = Some(d.as_string().unwrap());
                }
                if id.is_some() {
                    return Some(id.unwrap());
                }
            }
            None
        }
    });

    let mapped = filtered
        .chain(futures::stream::iter(input_args))
        .map(move |id| {
            let collection = collection.clone();
            async move { collection.remove(id, RemoveOptions::default()).await }
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
