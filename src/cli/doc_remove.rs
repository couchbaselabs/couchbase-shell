//! The `doc remove` command performs a KV remove operation.

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
use tokio::time::Instant;

pub struct DocRemove {
    state: Arc<Mutex<State>>,
}

impl DocRemove {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for DocRemove {
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
            .named("scope", SyntaxShape::String, "the name of the scope", None)
            .named(
                "collection",
                SyntaxShape::String,
                "the name of the collection",
                None,
            )
    }

    fn usage(&self) -> &str {
        "Removes a document through the data service"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        run_get(self.state.clone(), args)
    }
}

fn run_get(state: Arc<Mutex<State>>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();
    let args = args.evaluate_once()?;

    let id_column = args
        .call_info
        .args
        .get("id-column")
        .map(|id| id.as_string().ok())
        .flatten()
        .unwrap_or_else(|| String::from("id"));

    let guard = state.lock().unwrap();
    let active_cluster = guard.active_cluster();
    let (bucket, scope, collection) = namespace_from_args(&args, active_cluster)?;

    let input_args = if let Some(id) = args.nth(0) {
        vec![id.as_string()?]
    } else {
        vec![]
    };

    let filtered = args.input.filter_map(move |i| {
        let id_column = id_column.clone();

        if let UntaggedValue::Row(dict) = i.value {
            if let MaybeOwned::Borrowed(d) = dict.get_data(id_column.as_ref()) {
                return d.as_string().ok();
            }
        }
        None
    });

    let cluster = active_cluster.cluster();

    let mut client = cluster.key_value_client();

    let mut success = 0;
    let mut failed = 0;
    let mut fail_reasons: HashSet<String> = HashSet::new();
    for item in filtered.chain(input_args).into_iter() {
        let deadline = Instant::now().add(active_cluster.timeouts().data_timeout());
        let result = client
            .request(
                KeyValueRequest::Remove { key: item },
                bucket.clone(),
                scope.clone(),
                collection.clone(),
                deadline,
                ctrl_c.clone(),
            )
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
