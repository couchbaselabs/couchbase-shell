//! The `doc remove` command performs a KV remove operation.

use crate::state::State;

use crate::cli::doc_get::ids_from_input;
use crate::cli::doc_upsert::{
    build_batched_kv_items, prime_manifest_if_required, process_kv_workers,
};
use crate::cli::util::{cluster_identifiers_from, namespace_from_args};
use crate::client::KeyValueRequest;
use async_trait::async_trait;
use futures::stream::FuturesUnordered;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue};
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
            .named(
                "batch-size",
                SyntaxShape::Number,
                "the maximum number of items to batch send at a time",
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

fn run_get(state: Arc<Mutex<State>>, mut args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();

    let id_column = args
        .get_flag("id-column")?
        .unwrap_or_else(|| String::from("id"));

    let ids = ids_from_input(&mut args, id_column)?;
    let batch_size: Option<i32> = args.get_flag("batch-size")?;
    let mut all_ids: Vec<Vec<String>> = vec![];
    if let Some(size) = batch_size {
        all_ids = build_batched_kv_items(size as u32, ids.clone());
    }

    let bucket_flag = args.get_flag("bucket")?;
    let scope_flag = args.get_flag("scope")?;
    let collection_flag = args.get_flag("collection")?;

    let cluster_identifiers = cluster_identifiers_from(&state, &args, true)?;

    let guard = state.lock().unwrap();

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
        let deadline = Instant::now().add(active_cluster.timeouts().data_timeout());
        let mut client = rt.block_on(active_cluster.cluster().key_value_client(
            bucket.clone(),
            deadline,
            ctrl_c.clone(),
        ))?;

        prime_manifest_if_required(
            &rt,
            scope.clone(),
            collection.clone(),
            ctrl_c.clone(),
            Instant::now().add(active_cluster.timeouts().data_timeout()),
            &mut client,
        )?;

        if all_ids.is_empty() {
            all_ids = build_batched_kv_items(active_cluster.kv_batch_size(), ids.clone());
        }

        let client = Arc::new(client);

        let mut workers = FuturesUnordered::new();
        let mut success = 0;
        let mut failed = 0;
        let mut fail_reasons: HashSet<String> = HashSet::new();
        for items in all_ids.clone() {
            for item in items.clone() {
                let deadline = Instant::now().add(active_cluster.timeouts().data_timeout());
                let scope = scope.clone();
                let collection = collection.clone();
                let ctrl_c = ctrl_c.clone();

                let client = client.clone();

                workers.push(async move {
                    client
                        .request(
                            KeyValueRequest::Remove { key: item },
                            scope,
                            collection,
                            deadline,
                            ctrl_c,
                        )
                        .await
                });
            }

            let worked = process_kv_workers(workers, &rt);

            success += worked.success;
            failed += worked.failed;
            fail_reasons.extend(worked.fail_reasons);
            workers = FuturesUnordered::new()
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
