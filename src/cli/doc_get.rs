//! The `kv-get` command performs a KV get operation.

use super::util::convert_json_value_to_nu_value;
use crate::state::State;
use couchbase::GetOptions;

use async_trait::async_trait;
use futures::stream::StreamExt;
use log::debug;
use nu_cli::{CommandArgs, CommandRegistry, OutputStream};
use nu_errors::ShellError;
use nu_protocol::{
    MaybeOwned, Primitive, Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue,
};
use nu_source::Tag;
use std::sync::Arc;

pub struct DocGet {
    state: Arc<State>,
}

impl DocGet {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_cli::WholeStreamCommand for DocGet {
    fn name(&self) -> &str {
        "doc get"
    }

    fn signature(&self) -> Signature {
        Signature::build("doc get")
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
            .switch(
                "flatten",
                "If set, flattens the content into the toplevel",
                None,
            )
    }

    fn usage(&self) -> &str {
        "Fetches a document through the data service"
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

    let flatten = args.get("flatten").is_some();

    let bucket = state.active_cluster().bucket(&bucket_name);
    let collection = bucket.default_collection();

    debug!("Running kv get for docs {:?}", &ids);

    let mut results = vec![];
    for id in ids {
        match collection.get(&id, GetOptions::default()).await {
            Ok(res) => {
                let tag = Tag::default();
                let mut collected = TaggedDictBuilder::new(&tag);
                collected.insert_value(&id_column, id);
                collected.insert_value("cas", UntaggedValue::int(res.cas()).into_untagged_value());
                let content = res.content::<serde_json::Value>().unwrap();
                let content_converted = convert_json_value_to_nu_value(&content, Tag::default());
                if flatten {
                    if let UntaggedValue::Row(d) = content_converted.value {
                        for (k, v) in d.entries {
                            collected.insert_value(k, v);
                        }
                    }
                } else {
                    collected.insert_value("content", content_converted);
                }
                results.push(collected.into_value());
            }
            Err(_e) => {}
        };
    }
    Ok(OutputStream::from(results))
}
