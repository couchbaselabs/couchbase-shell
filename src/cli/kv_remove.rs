//! The `kv remove` command performs a KV remove operation.

use crate::state::State;
use couchbase::RemoveOptions;

use futures::stream::StreamExt;
use log::debug;
use nu_cli::{CommandArgs, CommandRegistry, OutputStream};
use nu_errors::ShellError;
use nu_protocol::{
    MaybeOwned, Primitive, Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue,
};
use nu_source::Tag;
use std::sync::Arc;
use async_trait::async_trait;

pub struct KvRemove {
    state: Arc<State>,
}

impl KvRemove {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_cli::WholeStreamCommand for KvRemove {
    fn name(&self) -> &str {
        "kv remove"
    }

    fn signature(&self) -> Signature {
        Signature::build("kv remove")
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
        "Removes a document through Key/Value"
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
    let mut args = args.evaluate_once(registry).await?;

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

    let mut ids = vec![];
    while let Some(item) = args.input.next().await {
        let untagged = item.into();
        match untagged {
            UntaggedValue::Primitive(p) => match p {
                Primitive::String(s) => ids.push(s.clone()),
                _ => {}
            },
            UntaggedValue::Row(d) => {
                if let MaybeOwned::Borrowed(d) = d.get_data(id_column.as_ref()) {
                    let untagged = &d.value;
                    if let UntaggedValue::Primitive(p) = untagged {
                        if let Primitive::String(s) = p {
                            ids.push(s.clone())
                        }
                    }
                }
            }
            _ => {}
        }
    }

    if let Some(id) = args.nth(0) {
        ids.push(id.as_string()?);
    }

    let bucket = state.active_cluster().bucket(&bucket_name);
    let collection = bucket.default_collection();

    debug!("Running kv remove for docs {:?}", &ids);

    let mut results = vec![];
    for id in ids {
        match collection.remove(&id, RemoveOptions::default()).await {
            Ok(res) => {
                let tag = Tag::default();
                let mut collected = TaggedDictBuilder::new(&tag);
                collected.insert_value(&id_column, id);
                collected.insert_value("cas", UntaggedValue::int(res.cas()).into_untagged_value());
                results.push(collected.into_value());
            }
            Err(_e) => {}
        };
    }
    Ok(OutputStream::from(results))
}
