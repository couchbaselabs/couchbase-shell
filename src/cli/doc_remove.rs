//! The `doc remove` command performs a KV remove operation.

use crate::cli::doc_common::{
    build_batched_kv_items, get_active_cluster_client_cid, process_kv_workers, MutationResult,
};
use crate::cli::doc_get::ids_from_input;
use crate::cli::util::cluster_identifiers_from;
use crate::client::KeyValueRequest;
use crate::state::State;
use futures::stream::FuturesUnordered;
use nu_engine::command_prelude::Call;
use nu_engine::CallExt;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};
use std::collections::HashSet;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;
use tokio::time::Instant;

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
            .switch("halt-on-error", "halt on any errors", Some('e'))
            .category(Category::Custom("couchbase".to_string()))
    }

    fn description(&self) -> &str {
        "Removes a document through the data service"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        run_remove(self.state.clone(), engine_state, stack, call, input)
    }

    fn requires_ast_for_arguments(&self) -> bool {
        true
    }
}

fn run_remove(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let signals = engine_state.signals().clone();

    let id_column = call
        .get_flag(engine_state, stack, "id-column")?
        .unwrap_or_else(|| String::from("id"));

    let ids = ids_from_input(input, id_column.clone(), call.positional_nth(stack, 0))?;
    let batch_size: Option<i64> = call.get_flag(engine_state, stack, "batch-size")?;
    let mut all_ids: Vec<Vec<String>> = vec![];
    if let Some(size) = batch_size {
        all_ids = build_batched_kv_items(size as u32, ids.clone());
    }

    let bucket_flag = call.get_flag(engine_state, stack, "bucket")?;
    let scope_flag = call.get_flag(engine_state, stack, "scope")?;
    let collection_flag = call.get_flag(engine_state, stack, "collection")?;
    let halt_on_error = call.has_flag(engine_state, stack, "halt-on-error")?;

    let cluster_identifiers = cluster_identifiers_from(engine_state, stack, &state, call, true)?;

    let guard = state.lock().unwrap();

    let mut results = vec![];
    for identifier in cluster_identifiers {
        let rt = Runtime::new().unwrap();
        let (active_cluster, client, cid) = match get_active_cluster_client_cid(
            &rt,
            identifier.clone(),
            &guard,
            bucket_flag.clone(),
            scope_flag.clone(),
            collection_flag.clone(),
            signals.clone(),
            span,
        ) {
            Ok(c) => c,
            Err(e) => {
                if halt_on_error {
                    return Err(e);
                }

                let mut failures = HashSet::new();
                failures.insert(e.to_string());
                let collected = MutationResult::new(identifier.clone())
                    .fail_reasons(failures)
                    .into_value(call.head);
                results.push(collected);
                continue;
            }
        };

        if all_ids.is_empty() {
            all_ids = build_batched_kv_items(active_cluster.kv_batch_size(), ids.clone());
        }

        let mut workers = FuturesUnordered::new();
        let mut success = 0;
        let mut failed = 0;
        let mut fail_reasons: HashSet<String> = HashSet::new();
        for items in all_ids.clone() {
            for item in items.clone() {
                let deadline = Instant::now().add(active_cluster.timeouts().data_timeout());
                let signal = signals.clone();
                let client = client.clone();

                workers.push(async move {
                    client
                        .request(KeyValueRequest::Remove { key: item }, cid, deadline, signal)
                        .await
                });
            }

            let worked = process_kv_workers(workers, &rt, halt_on_error, span)?;

            success += worked.success;
            failed += worked.failed;
            fail_reasons.extend(worked.fail_reasons);
            workers = FuturesUnordered::new()
        }

        let collected = MutationResult::new(identifier.clone())
            .success(success)
            .failed(failed)
            .fail_reasons(fail_reasons);

        results.push(collected.into_value(span));
    }

    Ok(Value::List {
        vals: results,
        internal_span: span,
    }
    .into_pipeline_data())
}
