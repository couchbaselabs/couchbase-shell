//! The `doc upsert` command performs a KV upsert operation.

use super::util::convert_nu_value_to_json_value;

use crate::state::State;

use crate::client::KeyValueRequest;
use async_trait::async_trait;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{MaybeOwned, Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue};
use nu_source::Tag;
use nu_stream::OutputStream;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;

pub struct DocUpsert {
    state: Arc<State>,
}

impl DocUpsert {
    pub fn new(state: Arc<State>) -> Self {
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

fn run_upsert(state: Arc<State>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once()?;
    // let ctrl_c = args.ctrl_c.clone();

    let id_column = args
        .call_info
        .args
        .get("id-column")
        .map(|id| id.as_string().ok())
        .flatten()
        .unwrap_or_else(|| String::from("id"));

    let content_column = args
        .call_info
        .args
        .get("content-column")
        .map(|content| content.as_string().ok())
        .flatten()
        .unwrap_or_else(|| String::from("content"));

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

    let expiry_arg = args
        .call_info
        .args
        .get("expiry")
        .map(|e| e.as_u32().unwrap_or_else(|_| 0));

    let expiry = expiry_arg.unwrap_or(0);

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
    let cluster = active_cluster.cluster();
    let mut client = cluster.key_value_client(
        active_cluster.username().into(),
        active_cluster.password().into(),
        bucket.clone(),
        scope.clone(),
        collection.clone(),
    )?;
    let timeout = match active_cluster.timeouts().data_timeout() {
        Some(t) => t.clone(),
        None => Duration::from_millis(2500),
    };

    let rt = Runtime::new().unwrap();
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

        let result = rt
            .block_on(client.request(
                KeyValueRequest::Set {
                    key: item.0,
                    value,
                    expiry: expiry.clone(),
                },
                timeout.clone(),
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
