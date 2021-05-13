//! The `doc get` command performs a KV get operation.

use super::util::convert_json_value_to_nu_value;
use crate::state::State;

use crate::client::KeyValueRequest;
use async_trait::async_trait;
use log::debug;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{
    MaybeOwned, Primitive, Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue, Value,
};
use nu_source::Tag;
use nu_stream::OutputStream;
use std::sync::Arc;
use tokio::runtime::Runtime;

pub struct DocGet {
    state: Arc<State>,
}

impl DocGet {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for DocGet {
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

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        run_get(self.state.clone(), args)
    }
}

fn run_get(state: Arc<State>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();
    let mut args = args.evaluate_once()?;

    let id_column = args
        .call_info
        .args
        .get("id-column")
        .map(|id| id.as_string().ok())
        .flatten()
        .unwrap_or_else(|| String::from("id"));

    let mut ids = vec![];
    while let Some(item) = args.input.next() {
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

    let active_cluster = state.active_cluster();
    let bucket = match args
        .call_info
        .args
        .get("bucket")
        .map(|bucket| bucket.as_string().ok())
        .flatten()
        .or_else(|| active_cluster.active_bucket())
    {
        Some(v) => Ok(v),
        None => Err(ShellError::untagged_runtime_error(format!(
            "Could not auto-select a bucket - please use --bucket instead"
        ))),
    }?;

    let scope = match args
        .call_info
        .args
        .get("scope")
        .map(|c| c.as_string().ok())
        .flatten()
    {
        Some(s) => s,
        None => match active_cluster.active_scope() {
            Some(s) => s,
            None => "".into(),
        },
    };

    let collection = match args
        .call_info
        .args
        .get("collection")
        .map(|c| c.as_string().ok())
        .flatten()
    {
        Some(c) => c,
        None => match active_cluster.active_collection() {
            Some(c) => c,
            None => "".into(),
        },
    };

    debug!("Running kv get for docs {:?}", &ids);

    let cluster = active_cluster.cluster();

    let mut results: Vec<Value> = vec![];
    let rt = Runtime::new().unwrap();
    let mut client = cluster.key_value_client(
        active_cluster.username().into(),
        active_cluster.password().into(),
        bucket.clone(),
        scope.clone(),
        collection.clone(),
    )?;
    for id in ids {
        let response = rt.block_on(client.request(KeyValueRequest::Get { key: id.clone() }));

        match response {
            Ok(mut res) => {
                let tag = Tag::default();
                let mut collected = TaggedDictBuilder::new(&tag);
                collected.insert_value(&id_column, id.clone());
                collected.insert_value("cas", UntaggedValue::int(res.cas()).into_untagged_value());
                let content = res.content().unwrap();
                let content_converted = convert_json_value_to_nu_value(&content, Tag::default())?;
                collected.insert_value("content", content_converted);
                collected.insert_value("error", "".to_string());
                results.push(collected.into_value());
            }
            Err(e) => {
                let tag = Tag::default();
                let mut collected = TaggedDictBuilder::new(&tag);
                collected.insert_value(&id_column, id.clone());
                collected.insert_value("cas", "".to_string());
                collected.insert_value("content", "".to_string());
                collected.insert_value("error", e.to_string());
                results.push(collected.into_value());
            }
        }
    }

    Ok(OutputStream::from(results))
}
