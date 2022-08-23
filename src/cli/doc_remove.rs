//! The `doc remove` command performs a KV remove operation.

use crate::state::State;

use crate::cli::doc_get::ids_from_input;
use crate::cli::doc_upsert::{
    build_batched_kv_items, prime_manifest_if_required, process_kv_workers,
};
use crate::cli::util::{
    cluster_identifiers_from, get_active_cluster, namespace_from_args, NuValueMap,
};
use crate::client::KeyValueRequest;
use futures::stream::FuturesUnordered;
use std::collections::HashSet;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;
use tokio::time::Instant;

use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct DocRemove {
    state: Arc<Mutex<State>>,
}

impl DocRemove {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for DocRemove {
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
            .category(Category::Custom("couchbase".to_string()))
    }

    fn usage(&self) -> &str {
        "Removes a document through the data service"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        run_get(self.state.clone(), engine_state, stack, call, input)
    }
}

fn run_get(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();

    let id_column = call
        .get_flag(engine_state, stack, "id-column")?
        .unwrap_or_else(|| String::from("id"));

    let ids = ids_from_input(call, input, id_column, ctrl_c.clone())?;
    let batch_size: Option<i64> = call.get_flag(engine_state, stack, "batch-size")?;
    let mut all_ids: Vec<Vec<String>> = vec![];
    if let Some(size) = batch_size {
        all_ids = build_batched_kv_items(size as u32, ids.clone());
    }

    let bucket_flag = call.get_flag(engine_state, stack, "bucket")?;
    let scope_flag = call.get_flag(engine_state, stack, "scope")?;
    let collection_flag = call.get_flag(engine_state, stack, "collection")?;

    let cluster_identifiers = cluster_identifiers_from(&engine_state, stack, &state, &call, true)?;

    let guard = state.lock().unwrap();

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

        let rt = Runtime::new().unwrap();
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

        let mut collected = NuValueMap::default();
        collected.add_i64("processed", (success + failed) as i64, span);
        collected.add_i64("success", success as i64, span);
        collected.add_i64("failed", failed as i64, span);

        let reasons = fail_reasons.into_iter().collect::<Vec<String>>().join(", ");
        collected.add_string("failures", reasons, span);
        collected.add_string("cluster", identifier.clone(), span);

        results.push(collected.into_value(span));
    }

    Ok(Value::List {
        vals: results,
        span,
    }
    .into_pipeline_data())
}
