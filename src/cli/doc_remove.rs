//! The `doc remove` command performs a KV remove operation.

use crate::state::State;

use crate::cli::util::{cluster_identifiers_from, namespace_from_args};
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
            .named(
                "clusters",
                SyntaxShape::String,
                "the clusters which should be contacted",
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

    let id_column = args
        .get_flag("id-column")?
        .unwrap_or_else(|| String::from("id"));

    let bucket_flag = args.get_flag("bucket")?;
    let scope_flag = args.get_flag("scope")?;
    let collection_flag = args.get_flag("collection")?;

    let cluster_identifiers = cluster_identifiers_from(&state, &args, true)?;

    let guard = state.lock().unwrap();

    let input_args = if let Some(id) = args.opt::<String>(0)? {
        vec![id]
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
    let mut all_items = vec![];
    for item in filtered.chain(input_args).into_iter() {
        all_items.push(item);
    }

    let mut results = vec![];
    for identifier in cluster_identifiers {
        let active_cluster = match guard.clusters().get(&identifier) {
            Some(c) => c,
            None => {
                return Err(ShellError::unexpected("Cluster not found"));
            }
        };

        let (bucket, scope, collection) = namespace_from_args(
            bucket_flag.clone(),
            scope_flag.clone(),
            collection_flag.clone(),
            active_cluster,
        )?;

        let rt = Runtime::new().unwrap();
        let mut client = active_cluster.cluster().key_value_client();

        let mut success = 0;
        let mut failed = 0;
        let mut fail_reasons: HashSet<String> = HashSet::new();
        for item in all_items.clone() {
            let deadline = Instant::now().add(active_cluster.timeouts().data_timeout());
            let result = rt
                .block_on(client.request(
                    KeyValueRequest::Remove { key: item },
                    bucket.clone(),
                    scope.clone(),
                    collection.clone(),
                    deadline,
                    ctrl_c.clone(),
                ))
                .map_err(|e| ShellError::unexpected(e.to_string()));

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
        collected.insert_value("cluster", identifier.clone());
        results.push(collected.into_value());
    }
    Ok(OutputStream::from(results))
}
