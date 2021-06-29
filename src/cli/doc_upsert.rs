//! The `doc upsert` command performs a KV upsert operation.

use super::util::convert_nu_value_to_json_value;

use crate::state::State;

use crate::cli::util::namespace_from_args;
use crate::client::KeyValueRequest;
use async_trait::async_trait;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{MaybeOwned, Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue};
use nu_source::Tag;
use nu_stream::OutputStream;
use std::collections::HashSet;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;
use tokio::time::Instant;

pub struct DocUpsert {
    state: Arc<Mutex<State>>,
}

impl DocUpsert {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for DocUpsert {
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

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        run_upsert(self.state.clone(), args)
    }
}

fn run_upsert(state: Arc<Mutex<State>>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();

    let id_column = args
        .get_flag("id-column")?
        .unwrap_or_else(|| String::from("id"));

    let content_column = args
        .get_flag("content-column")?
        .unwrap_or_else(|| String::from("content"));

    let guard = state.lock().unwrap();
    let active_cluster = guard.active_cluster();
    let (bucket, scope, collection) = namespace_from_args(&args, active_cluster)?;

    let expiry: i32 = args.get_flag("expiry")?.unwrap_or(0);

    let input_args = if let Some(id) = args.opt::<String>(0)? {
        if let Some(content) = args.opt::<String>(1)? {
            let content = serde_json::from_str(&content)?;
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
    });
    let rt = Runtime::new().unwrap();
    let mut client = active_cluster.cluster().key_value_client();

    let mut success = 0;
    let mut failed = 0;
    let mut fail_reasons: HashSet<String> = HashSet::new();
    for item in filtered.chain(input_args).into_iter() {
        let value = match serde_json::to_vec(&item.1) {
            Ok(v) => v,
            Err(e) => {
                return Err(ShellError::untagged_runtime_error(e.to_string()));
            }
        };

        let deadline = Instant::now().add(active_cluster.timeouts().data_timeout());
        let result = rt
            .block_on(client.request(
                KeyValueRequest::Set {
                    key: item.0,
                    value,
                    expiry: expiry as u32,
                },
                bucket.clone(),
                scope.clone(),
                collection.clone(),
                deadline,
                ctrl_c.clone(),
            ))
            .map_err(|e| ShellError::untagged_runtime_error(e.to_string()));

        match result {
            Ok(_) => success += 1,
            Err(e) => {
                failed += 1;
                fail_reasons.insert(e.to_string());
            }
        };
    }

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

    Ok(vec![collected.into_value()].into())
}
