//! The `doc upsert` command performs a KV upsert operation.

use super::util::convert_nu_value_to_json_value;

use crate::state::State;

use crate::cli::util::{cluster_identifiers_from, namespace_from_args};
use crate::client::{ClientError, KeyValueRequest, KvClient, KvResponse};
use async_trait::async_trait;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{MaybeOwned, Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue, Value};
use nu_source::Tag;
use nu_stream::OutputStream;
use std::collections::HashSet;
use std::future::Future;
use std::ops::Add;
use std::sync::atomic::AtomicBool;
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
        "Upsert (insert or override) a document through the data service"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        run_upsert(self.state.clone(), args)
    }
}

fn build_req(key: String, value: Vec<u8>, expiry: u32) -> KeyValueRequest {
    KeyValueRequest::Set { key, value, expiry }
}

fn run_upsert(state: Arc<Mutex<State>>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let results = run_kv_store_ops(state, args, build_req)?;

    Ok(OutputStream::from(results))
}

pub(crate) fn run_kv_store_ops(
    state: Arc<Mutex<State>>,
    args: CommandArgs,
    req_builder: fn(String, Vec<u8>, u32) -> KeyValueRequest,
) -> Result<Vec<Value>, ShellError> {
    let ctrl_c = args.ctrl_c();

    let id_column = args
        .get_flag("id-column")?
        .unwrap_or_else(|| String::from("id"));

    let content_column = args
        .get_flag("content-column")?
        .unwrap_or_else(|| String::from("content"));

    let expiry: i32 = args.get_flag("expiry")?.unwrap_or(0);
    let batch_size: Option<i32> = args.get_flag("batch-size")?;

    let bucket_flag = args.get_flag("bucket")?;
    let scope_flag = args.get_flag("scope")?;
    let collection_flag = args.get_flag("collection")?;

    let cluster_identifiers = cluster_identifiers_from(&state, &args, true)?;

    let guard = state.lock().unwrap();

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

    let mut all_items = vec![];
    for item in filtered.chain(input_args).into_iter() {
        let value = match serde_json::to_vec(&item.1) {
            Ok(v) => v,
            Err(e) => {
                return Err(ShellError::unexpected(e.to_string()));
            }
        };

        all_items.push((item.0, value));
    }

    let mut all_values = vec![];
    if let Some(size) = batch_size {
        all_values = build_batched_kv_items(size as u32, all_items.clone());
    }

    let rt = Runtime::new().unwrap();

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

        let client = Arc::new(client);

        if all_values.is_empty() {
            all_values = build_batched_kv_items(active_cluster.kv_batch_size(), all_items.clone());
        }

        let mut workers = FuturesUnordered::new();
        let mut success = 0;
        let mut failed = 0;
        let mut fail_reasons: HashSet<String> = HashSet::new();
        for items in all_values.clone() {
            for item in items.clone() {
                let deadline = Instant::now().add(active_cluster.timeouts().data_timeout());

                let scope = scope.clone();
                let collection = collection.clone();
                let ctrl_c = ctrl_c.clone();

                let client = client.clone();

                workers.push(async move {
                    client
                        .request(
                            req_builder(item.0, item.1, expiry as u32),
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

    Ok(results)
}

pub(crate) struct WorkerResponse {
    pub(crate) success: i32,
    pub(crate) failed: i32,
    pub(crate) fail_reasons: HashSet<String>,
}

pub(crate) fn process_kv_workers(
    mut workers: FuturesUnordered<impl Future<Output = Result<KvResponse, ClientError>>>,
    rt: &Runtime,
) -> WorkerResponse {
    let (success, failed, fail_reasons) = rt.block_on(async {
        let mut success = 0;
        let mut failed = 0;
        let mut fail_reasons: HashSet<String> = HashSet::new();
        while let Some(result) = workers.next().await {
            match result {
                Ok(_) => success += 1,
                Err(e) => {
                    failed += 1;
                    fail_reasons.insert(e.to_string());
                }
            }
        }
        (success, failed, fail_reasons)
    });

    WorkerResponse {
        success,
        failed,
        fail_reasons,
    }
}

pub(crate) fn build_batched_kv_items<T>(
    batch_size: u32,
    items: impl IntoIterator<Item = T>,
) -> Vec<Vec<T>> {
    let mut all_items = vec![];
    let mut these_items = vec![];
    let mut i = 0;
    for item in items.into_iter() {
        these_items.push(item);
        if i == batch_size {
            all_items.push(these_items);
            these_items = vec![];
            i = 0;
            continue;
        }

        i += 1;
    }
    all_items.push(these_items);

    all_items
}

pub(crate) fn prime_manifest_if_required(
    rt: &Runtime,
    scope: String,
    collection: String,
    ctrl_c: Arc<AtomicBool>,
    deadline: Instant,
    client: &mut KvClient,
) -> Result<(), ShellError> {
    if KvClient::is_non_default_scope_collection(scope, collection) {
        rt.block_on(client.fetch_collections_manifest(deadline, ctrl_c))
            .map_err(|e| ShellError::unexpected(e.to_string()))?;
    }

    Ok(())
}
