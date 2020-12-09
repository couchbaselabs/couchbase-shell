//! The `doc get` command performs a KV get operation.

use super::util::{convert_json_value_to_nu_value, couchbase_error_to_shell_error};
use crate::state::State;
use couchbase::GetOptions;

use crate::cli::util::{collection_from_args, run_interruptable};
use async_trait::async_trait;
use futures::stream::StreamExt;
use log::debug;
use nu_cli::{CommandArgs, CommandRegistry, OutputStream};
use nu_errors::ShellError;
use nu_protocol::{
    MaybeOwned, Primitive, ReturnSuccess, ReturnValue, Signature, SyntaxShape, TaggedDictBuilder,
    UntaggedValue, Value,
};
use nu_source::{PrettyDebug, Tag};
use std::collections::HashMap;
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
            .named("scope", SyntaxShape::String, "the name of the scope", None)
            .named(
                "collection",
                SyntaxShape::String,
                "the name of the collection",
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
    let ctrl_c = args.ctrl_c.clone();

    let id_column = args
        .get("id-column")
        .map(|id| id.as_string().ok())
        .flatten()
        .unwrap_or_else(|| String::from("id"));

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

    let collection = match collection_from_args(&args, &state) {
        Ok(c) => c,
        Err(e) => {
            return Err(e);
        }
    };

    debug!("Running kv get for docs {:?}", &ids);

    let mut results: Vec<ReturnValue> = vec![];
    for id in ids {
        let get = collection.get(&id, GetOptions::default());

        match run_interruptable(get, ctrl_c.clone()).await {
            Ok(res) => {
                let tag = Tag::default();
                let mut collected = TaggedDictBuilder::new(&tag);
                collected.insert_value(&id_column, id.clone());
                collected.insert_value("cas", UntaggedValue::int(res.cas()).into_untagged_value());
                let content = res
                    .content::<serde_json::Value>()
                    .map_err(|e| couchbase_error_to_shell_error(e))?;
                let content_converted = convert_json_value_to_nu_value(&content, Tag::default())?;
                if flatten {
                    let flattened = do_flatten(content_converted.value);
                    for (k, v) in flattened {
                        collected.insert_value(k, v);
                    }
                } else {
                    collected.insert_value("content", content_converted);
                }
                collected.insert_value("error", "".to_string());
                results.push(Ok(ReturnSuccess::Value(collected.into_value())));
            }
            Err(e) => {
                let tag = Tag::default();
                let mut collected = TaggedDictBuilder::new(&tag);
                collected.insert_value(&id_column, id.clone());
                collected.insert_value("cas", "".to_string());
                collected.insert_value("content", "".to_string());
                collected.insert_value("error", e.display());
                results.push(Ok(ReturnSuccess::Value(collected.into_value())));
            }
        }
    }
    Ok(OutputStream::from(results))
}

fn do_flatten(val: UntaggedValue) -> HashMap<String, Value> {
    let mut collected = HashMap::new();
    match val {
        UntaggedValue::Row(d) => {
            for (k, v) in d.entries {
                match v.value {
                    UntaggedValue::Row(r) => {
                        let inner_collected = do_flatten(UntaggedValue::Row(r));
                        for (k, v) in inner_collected {
                            collected.insert(k, v);
                        }
                    }
                    _ => {
                        collected.insert(k, v);
                    }
                }
            }
        }
        _ => {}
    }

    collected
}
