//! The `doc upsert` command performs a KV upsert operation.

use super::util::convert_nu_value_to_json_value;
use crate::cli::error::{generic_error, serialize_error};
use crate::cli::util::{
    cluster_identifiers_from, get_active_cluster, namespace_from_args, NuValueMap,
};
use crate::client::{ClientError, KeyValueRequest, KvClient, KvResponse};
use crate::state::State;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};
use std::collections::HashSet;
use std::future::Future;
use std::ops::Add;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;
use tokio::time::Instant;

#[derive(Clone)]
pub struct DocUpsert {
    state: Arc<Mutex<State>>,
}

impl DocUpsert {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for DocUpsert {
    fn name(&self) -> &str {
        "doc upsert"
    }

    fn signature(&self) -> Signature {
        Signature::build("doc upsert")
            .optional("id", SyntaxShape::String, "the document id")
            .optional("content", SyntaxShape::Any, "the document content")
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
            .category(Category::Custom("couchbase".into()))
    }

    fn usage(&self) -> &str {
        "Upsert (insert or override) a document through the data service"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        run_upsert(self.state.clone(), engine_state, stack, call, input)
    }
}

fn build_req(key: String, value: Vec<u8>, expiry: u32) -> KeyValueRequest {
    KeyValueRequest::Set { key, value, expiry }
}

fn run_upsert(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let results = run_kv_store_ops(state, engine_state, stack, call, input, build_req)?;

    Ok(Value::List {
        vals: results,
        span: call.head,
    }
    .into_pipeline_data())
}

pub(crate) fn run_kv_store_ops(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
    req_builder: fn(String, Vec<u8>, u32) -> KeyValueRequest,
) -> Result<Vec<Value>, ShellError> {
    let span = call.head;
    let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();

    let id_column = call
        .get_flag(engine_state, stack, "id-column")?
        .unwrap_or_else(|| String::from("id"));

    let content_column = call
        .get_flag(engine_state, stack, "content-column")?
        .unwrap_or_else(|| String::from("content"));

    let expiry: i64 = call.get_flag(engine_state, stack, "expiry")?.unwrap_or(0);
    let batch_size: Option<i64> = call.get_flag(engine_state, stack, "batch-size")?;

    let bucket_flag = call.get_flag(engine_state, stack, "bucket")?;
    let scope_flag = call.get_flag(engine_state, stack, "scope")?;
    let collection_flag = call.get_flag(engine_state, stack, "collection")?;

    let cluster_identifiers = cluster_identifiers_from(&engine_state, stack, &state, &call, true)?;

    let guard = state.lock().unwrap();

    let input_args = if let Some(id) = call.opt::<String>(engine_state, stack, 0)? {
        if let Some(v) = call.opt::<Value>(engine_state, stack, 1)? {
            let content = convert_nu_value_to_json_value(&v, span)?;
            vec![(id, content)]
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    let filtered = input.into_iter().filter_map(move |i| {
        let id_column = id_column.clone();
        let content_column = content_column.clone();

        if let Value::Record { cols, vals, .. } = i {
            let mut id = None;
            let mut content = None;
            for (k, v) in cols.iter().zip(vals) {
                if k.clone() == id_column {
                    id = v.as_string().ok();
                }
                if k.clone() == content_column {
                    content = convert_nu_value_to_json_value(&v, span).ok();
                }
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
        let value =
            serde_json::to_vec(&item.1).map_err(|e| serialize_error(e.to_string(), span))?;

        all_items.push((item.0, value));
    }

    let mut all_values = vec![];
    if let Some(size) = batch_size {
        all_values = build_batched_kv_items(size as u32, all_items.clone());
    }

    let rt = Runtime::new().unwrap();

    let mut results = vec![];
    for identifier in cluster_identifiers {
        let active_cluster = get_active_cluster(identifier.clone(), &guard, span.clone())?;

        let (bucket, scope, collection) = namespace_from_args(
            bucket_flag.clone(),
            scope_flag.clone(),
            collection_flag.clone(),
            active_cluster,
            span,
        )?;
        let deadline = Instant::now().add(active_cluster.timeouts().data_timeout());
        let mut client = rt.block_on(active_cluster.cluster().key_value_client(
            bucket.clone(),
            deadline,
            ctrl_c.clone(),
            span,
        ))?;

        prime_manifest_if_required(
            &rt,
            scope.clone(),
            collection.clone(),
            ctrl_c.clone(),
            Instant::now().add(active_cluster.timeouts().data_timeout()),
            &mut client,
            span.clone(),
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

        let mut collected = NuValueMap::default();
        collected.add_i64("processed", (success + failed) as i64, span);
        collected.add_i64("success", success as i64, span);
        collected.add_i64("failed", failed as i64, span);

        let reasons = fail_reasons.into_iter().collect::<Vec<String>>().join(", ");
        collected.add_string("failures", reasons, span);
        collected.add_string("cluster", identifier.clone(), span);

        results.push(collected.into_value(span));
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
    span: Span,
) -> Result<(), ShellError> {
    if KvClient::is_non_default_scope_collection(scope, collection) {
        rt.block_on(client.fetch_collections_manifest(deadline, ctrl_c))
            .map_err(|e| {
                generic_error(
                    format!("Failed to fetch collections manifest {}", e.to_string()),
                    None,
                    span,
                )
            })?;
    }

    Ok(())
}
