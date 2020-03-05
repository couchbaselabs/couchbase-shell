//! The `kv-get` command performs a KV get operation.

use super::util::convert_json_value_to_nu_value;
use crate::state::State;
use couchbase::GetOptions;

use futures::executor::block_on;
use futures::stream::StreamExt;
use log::debug;
use nu::{CommandArgs, CommandRegistry, OutputStream};
use nu_errors::ShellError;
use nu_protocol::{
    MaybeOwned, Primitive, Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue,
};
use nu_source::Tag;
use std::sync::Arc;

pub struct Get {
    state: Arc<State>,
}

impl Get {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

impl nu::WholeStreamCommand for Get {
    fn name(&self) -> &str {
        "kv-get"
    }

    fn signature(&self) -> Signature {
        Signature::build("kv-get").optional("id", SyntaxShape::String, "the document id")
    }

    fn usage(&self) -> &str {
        "Fetches a document through Key/Value"
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        block_on(run_get(self.state.clone(), args, registry))
    }
}

async fn run_get(
    state: Arc<State>,
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let mut args = args.evaluate_once(registry)?;

    let mut ids = vec![];
    while let Some(item) = args.input.next().await {
        let untagged = item.into();
        match untagged {
            UntaggedValue::Primitive(p) => match p {
                Primitive::String(s) => ids.push(s.clone()),
                _ => {}
            },
            UntaggedValue::Row(d) => {
                if let MaybeOwned::Borrowed(d) = d.get_data("id") {
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

    let bucket = state.active_cluster().cluster().bucket("travel-sample");
    let collection = bucket.default_collection();

    debug!("Running kv get for docs {:?}", &ids);

    let mut results = vec![];
    for id in ids {
        match collection.get(&id, GetOptions::default()).await {
            Ok(res) => {
                let tag = Tag::default();
                let mut collected = TaggedDictBuilder::new(&tag);
                collected.insert_value("id", id);
                collected.insert_value("cas", UntaggedValue::int(res.cas()).into_untagged_value());
                let content = res.content::<serde_json::Value>().unwrap();
                let content_converted = convert_json_value_to_nu_value(&content, Tag::default());
                collected.insert_value("content", content_converted);
                results.push(collected.into_value());
            }
            Err(_e) => {}
        };
    }
    Ok(OutputStream::from(results))
}
