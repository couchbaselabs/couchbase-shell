use crate::cli::util::{
    cluster_identifiers_from, convert_nu_value_to_json_value, get_active_cluster, NuValueMap,
};
use crate::cli::{client_error_to_shell_error, no_active_bucket_error, serialize_error};
use crate::client::connection_client::ConnectionClient;
use crate::client::ClientError;
use crate::remote_cluster::RemoteCluster;
use crate::state::State;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use log::info;
use nu_engine::command_prelude::Call;
use nu_engine::CallExt;
use nu_protocol::engine::{EngineState, Stack};
use nu_protocol::{PipelineData, ShellError, Signals, Span, Value};
use std::collections::HashSet;
use std::future::Future;
use std::ops::Add;
use std::sync::{Arc, Mutex, MutexGuard};
use tokio::runtime::Runtime;
use tokio::time::Instant;

pub(crate) fn run_kv_store_ops<Fut>(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
    run_cb: impl Fn(
        Arc<ConnectionClient>,
        String,
        String,
        String,
        Vec<u8>,
        u32,
        Instant,
        Signals,
    ) -> Fut,
) -> Result<Vec<Value>, ShellError>
where
    Fut: Future<Output = Result<(), ClientError>>,
{
    let span = call.head;

    let id_column = call
        .get_flag(engine_state, stack, "id-column")?
        .unwrap_or_else(|| String::from("id"));

    let content_column = call
        .get_flag(engine_state, stack, "content-column")?
        .unwrap_or_else(|| String::from("content"));

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

        if let Value::Record { val, .. } = i {
            let mut id = None;
            let mut content = None;
            for (k, v) in val.iter() {
                if k.clone() == id_column {
                    id = id_from_value(v, span);
                }
                if k.clone() == content_column {
                    content = convert_nu_value_to_json_value(v, span).ok();
                }
            }

            if let Some(c) = content {
                return Some((id.unwrap_or("".into()), c));
            }
        }
        None
    });

    let mut all_items = vec![];
    for item in filtered.chain(input_args) {
        let value =
            serde_json::to_vec(&item.1).map_err(|e| serialize_error(e.to_string(), span))?;

        all_items.push((item.0, value));
    }

    run_kv_mutations(state, engine_state, stack, call, span, all_items, run_cb)
}

pub fn id_from_value(v: &Value, span: Span) -> Option<String> {
    match v {
        Value::String { val, .. } => Some(val.clone()),
        Value::Int { val, .. } => Some(val.to_string()),
        _ => {
            info!(
                "Skipping doc with id '{}' as id is not an int or string",
                convert_nu_value_to_json_value(v, span).unwrap_or("error parsing id".into())
            );
            None
        }
    }
}

pub fn run_kv_mutations<Fut>(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    span: Span,
    all_items: Vec<(String, Vec<u8>)>,
    run_cb: impl Fn(
        Arc<ConnectionClient>,
        String,
        String,
        String,
        Vec<u8>,
        u32,
        Instant,
        Signals,
    ) -> Fut,
) -> Result<Vec<Value>, ShellError>
where
    Fut: Future<Output = Result<(), ClientError>>,
{
    let signals = engine_state.signals().clone();

    let expiry: i64 = call.get_flag(engine_state, stack, "expiry")?.unwrap_or(0);
    let batch_size: Option<i64> = call.get_flag(engine_state, stack, "batch-size")?;

    let bucket_flag = call.get_flag(engine_state, stack, "bucket")?;
    let scope_flag = call.get_flag(engine_state, stack, "scope")?;
    let collection_flag = call.get_flag(engine_state, stack, "collection")?;

    let halt_on_error = call.has_flag(engine_state, stack, "halt-on-error")?;

    let cluster_identifiers = cluster_identifiers_from(engine_state, stack, &state, call, true)?;

    let guard = state.lock().unwrap();

    let scope = scope_flag
        .or_else(|| {
            let active_cluster = guard.active_cluster()?;
            active_cluster.active_scope()
        })
        .unwrap_or("_default".to_string());

    let collection = collection_flag
        .or_else(|| {
            let active_cluster = guard.active_cluster()?;
            active_cluster.active_collection()
        })
        .unwrap_or("_default".to_string());

    let mut all_values = vec![];
    if let Some(size) = batch_size {
        all_values = build_batched_kv_items(size as u32, all_items.clone());
    }

    let mut results = vec![];
    for identifier in cluster_identifiers {
        let rt = Runtime::new().unwrap();
        let (active_cluster, client) = match get_active_cluster_connection_client(
            &rt,
            identifier.clone(),
            &guard,
            bucket_flag.clone(),
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

                let signals = signals.clone();

                let client = client.clone();

                let scope = scope.clone();
                let collection = collection.clone();

                if !item.0.is_empty() {
                    workers.push(run_cb(
                        client,
                        item.0,
                        scope,
                        collection,
                        item.1,
                        expiry as u32,
                        deadline,
                        signals,
                    ));
                } else {
                    failed += 1;
                    let mut missing_reason = HashSet::new();
                    missing_reason.insert("Missing doc id".into());
                    fail_reasons.extend(missing_reason);
                }
            }
            // process_kv_workers will handle creating an error for us if halt_on_error is set so we
            // can just bubble it.
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

    Ok(results)
}

pub(crate) struct WorkerResponse {
    pub(crate) success: i32,
    pub(crate) failed: i32,
    pub(crate) fail_reasons: HashSet<String>,
}

pub(crate) fn process_kv_workers(
    mut workers: FuturesUnordered<impl Future<Output = Result<(), ClientError>>>,
    rt: &Runtime,
    halt_on_error: bool,
    span: Span,
) -> Result<WorkerResponse, ShellError> {
    let (success, failed, fail_reasons) = rt.block_on(async {
        let mut success = 0;
        let mut failed = 0;
        let mut fail_reasons: HashSet<String> = HashSet::new();
        while let Some(result) = workers.next().await {
            match result {
                Ok(_) => success += 1,
                Err(e) => {
                    if halt_on_error {
                        return Err(client_error_to_shell_error(e, span));
                    }
                    failed += 1;
                    fail_reasons.insert(e.to_string());
                }
            }
        }
        Ok((success, failed, fail_reasons))
    })?;

    Ok(WorkerResponse {
        success,
        failed,
        fail_reasons,
    })
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

pub(crate) fn get_active_cluster_connection_client<'a>(
    rt: &Runtime,
    cluster: String,
    guard: &'a MutexGuard<State>,
    bucket: Option<String>,
    signals: Signals,
    span: Span,
) -> Result<(&'a RemoteCluster, Arc<ConnectionClient>), ShellError> {
    let active_cluster = get_active_cluster(cluster, guard, span)?;

    let bucket = match bucket.or_else(|| active_cluster.active_bucket()) {
        Some(v) => Ok(v),
        None => Err(no_active_bucket_error(span)),
    }?;

    let deadline = Instant::now().add(active_cluster.timeouts().data_timeout());
    let client = rt
        .block_on(
            active_cluster
                .cluster()
                .connection_client(bucket, deadline, signals),
        )
        .map_err(|e| client_error_to_shell_error(e, span))?;

    Ok((active_cluster, Arc::new(client)))
}

#[derive(Debug)]
pub struct MutationResult {
    success: i32,
    failed: i32,
    fail_reasons: HashSet<String>,
    cluster: String,
}

impl MutationResult {
    pub fn new(cluster: String) -> Self {
        Self {
            success: 0,
            failed: 0,
            fail_reasons: Default::default(),
            cluster,
        }
    }
    pub fn success(mut self, success: i32) -> Self {
        self.success = success;
        self
    }

    pub fn failed(mut self, failed: i32) -> Self {
        self.failed = failed;
        self
    }

    pub fn fail_reasons(mut self, fail_reasons: HashSet<String>) -> Self {
        self.fail_reasons = fail_reasons;
        self
    }

    pub fn into_value(self, span: Span) -> Value {
        let mut collected = NuValueMap::default();
        collected.add_i64("processed", (self.success + self.failed) as i64, span);
        collected.add_i64("success", self.success as i64, span);
        collected.add_i64("failed", self.failed as i64, span);

        let reasons = self
            .fail_reasons
            .into_iter()
            .collect::<Vec<String>>()
            .join(", ");
        collected.add_string("failures", reasons, span);
        collected.add_string("cluster", self.cluster, span);
        collected.into_value(span)
    }
}
