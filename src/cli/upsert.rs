//! The `kv-upsert` command performs a KV upsert operation.

use super::util::convert_nu_value_to_json_value;

use crate::state::State;
use couchbase::UpsertOptions;

use futures::executor::block_on;
use futures::stream::StreamExt;
use log::debug;
use nu_cli::{CommandArgs, CommandRegistry, OutputStream};
use nu_errors::ShellError;
use nu_protocol::{
    MaybeOwned, Primitive, Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue,
};
use nu_source::Tag;
use std::collections::HashMap;
use std::sync::Arc;

pub struct Upsert {
    state: Arc<State>,
}

impl Upsert {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

impl nu_cli::WholeStreamCommand for Upsert {
    fn name(&self) -> &str {
        "kv-upsert"
    }

    fn signature(&self) -> Signature {
        Signature::build("kv-upsert")
            .optional("id", SyntaxShape::String, "the document id")
            .optional("content", SyntaxShape::String, "the document content")
            .named(
                "id-column",
                SyntaxShape::String,
                "the name of the id column if used with an input stream",
                None,
            )
            .named(
                "content-column",
                SyntaxShape::String,
                "the name of the content column if used with an input stream",
                None,
            )
    }

    fn usage(&self) -> &str {
        "Upsert a document through Key/Value"
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        block_on(run_upsert(self.state.clone(), args, registry))
    }
}

async fn run_upsert(
    state: Arc<State>,
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let mut args = args.evaluate_once(registry)?;

    let mut rows = HashMap::new();

    let id_column = args
        .get("id-column")
        .map(|id| id.as_string().unwrap())
        .unwrap_or_else(|| String::from("id"));

    let content_column = args
        .get("content-column")
        .map(|content| content.as_string().unwrap())
        .unwrap_or_else(|| String::from("content"));

    while let Some(item) = args.input.next().await {
        let untagged = item.into();
        match untagged {
            UntaggedValue::Row(d) => {
                let mut id = String::from("");
                if let MaybeOwned::Borrowed(d) = d.get_data(id_column.as_ref()) {
                    let untagged = &d.value;
                    if let UntaggedValue::Primitive(p) = untagged {
                        if let Primitive::String(s) = p {
                            id = s.clone()
                        }
                    }
                }

                if id == "" {
                    continue;
                }

                if let MaybeOwned::Borrowed(d) = d.get_data(content_column.as_ref()) {
                    let untagged = &d.value;
                    match untagged {
                        UntaggedValue::Primitive(p) => {
                            if let Primitive::String(s) = p {
                                let content = serde_json::to_value(s)?;
                                rows.insert(id, content);
                            }
                        }
                        UntaggedValue::Row(_) => {
                            rows.insert(id, convert_nu_value_to_json_value(d)?);
                        }
                        UntaggedValue::Table(_) => {
                            rows.insert(id, convert_nu_value_to_json_value(d)?);
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    let mut arg_id = String::from("");
    if let Some(id) = args.nth(0) {
        arg_id = id.as_string()?;
    }

    if arg_id != "" {
        let mut arg_content = serde_json::to_value("")?;
        if let Some(content) = args.nth(1) {
            arg_content = convert_nu_value_to_json_value(&content)?;
        }

        // An empty value is a legitimate document
        rows.insert(arg_id, arg_content);
    }

    let bucket = state.active_cluster().cluster().bucket("travel-sample");
    let collection = bucket.default_collection();

    debug!("Running kv upsert for docs {:?}", &rows);

    let mut results = vec![];
    for (id, content) in rows.iter() {
        match collection
            .upsert(id, content, UpsertOptions::default())
            .await
        {
            Ok(_) => {
                let tag = Tag::default();
                let mut collected = TaggedDictBuilder::new(&tag);
                collected.insert_value(&id_column, id.clone());
                results.push(collected.into_value());
            }
            Err(e) => {
                debug!("Error received running upsert {:?}", e);
            }
        };
    }
    Ok(OutputStream::from(results))
}
