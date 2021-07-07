//! The `doc get` command performs a KV get operation.

use super::util::convert_json_value_to_nu_value;
use crate::state::State;

use crate::client::KeyValueRequest;
use log::debug;
use nu_engine::{CommandArgs, Example};
use nu_errors::ShellError;
use nu_protocol::{
    MaybeOwned, Primitive, Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue, Value,
};
use nu_source::Tag;
use nu_stream::OutputStream;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;
use tokio::time::Instant;

pub struct DocGet {
    state: Arc<Mutex<State>>,
}

impl DocGet {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

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

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Fetches a single document with the ID as an argument",
                example: "doc get my_doc_id",
                result: None,
            },
            Example {
                description: "Fetches multiple documents with IDs from the previous command",
                example: "echo [[id]; [airline_10] [airline_11]] | doc get",
                result: None,
            },
        ]
    }
}

fn run_get(state: Arc<Mutex<State>>, mut args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();

    let id_column: String = args.get_flag("id-column")?.unwrap_or_else(|| "id".into());
    let mut ids = vec![];
    while let Some(item) = args.input.next() {
        let untagged = item.into();
        match untagged {
            UntaggedValue::Primitive(Primitive::String(s)) => ids.push(s.clone()),
            UntaggedValue::Row(d) => {
                if let MaybeOwned::Borrowed(d) = d.get_data(id_column.as_ref()) {
                    let untagged = &d.value;
                    if let UntaggedValue::Primitive(Primitive::String(s)) = untagged {
                        ids.push(s.clone())
                    }
                }
            }
            _ => {}
        }
    }
    if let Some(id) = args.opt(0)? {
        ids.push(id);
    }

    let guard = state.lock().unwrap();
    let active_cluster = guard.active_cluster();
    let bucket = match args
        .get_flag("bucket")?
        .or_else(|| active_cluster.active_bucket())
    {
        Some(v) => Ok(v),
        None => Err(ShellError::untagged_runtime_error(
            "Could not auto-select a bucket - please use --bucket instead".to_string(),
        )),
    }?;

    let scope = match args.get_flag("scope")? {
        Some(s) => s,
        None => match active_cluster.active_scope() {
            Some(s) => s,
            None => "".into(),
        },
    };

    let collection = match args.get_flag("collection")? {
        Some(c) => c,
        None => match active_cluster.active_collection() {
            Some(c) => c,
            None => "".into(),
        },
    };

    debug!("Running kv get for docs {:?}", &ids);

    let rt = Runtime::new().unwrap();
    let mut results: Vec<Value> = vec![];
    let mut client = active_cluster.cluster().key_value_client();
    for id in ids {
        let deadline = Instant::now().add(active_cluster.timeouts().data_timeout());
        let response = rt.block_on(client.request(
            KeyValueRequest::Get { key: id.clone() },
            bucket.clone(),
            scope.clone(),
            collection.clone(),
            deadline,
            ctrl_c.clone(),
        ));

        match response {
            Ok(mut res) => {
                let tag = Tag::default();
                let mut collected = TaggedDictBuilder::new(&tag);
                collected.insert_value(&id_column, id.clone());
                collected.insert_value(
                    "cas",
                    UntaggedValue::int(res.cas() as i64).into_untagged_value(),
                );
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
