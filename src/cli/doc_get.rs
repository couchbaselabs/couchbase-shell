//! The `doc get` command performs a KV get operation.

use super::util::convert_json_value_to_nu_value;
use crate::state::State;

use crate::cli::doc_upsert::{build_batched_kv_items, prime_manifest_if_required};
use crate::cli::util::{
    cluster_identifiers_from, get_active_cluster, namespace_from_args, NuValueMap,
};
use crate::client::KeyValueRequest;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use log::debug;
use std::ops::Add;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;
use tokio::time::Instant;

use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct DocGet {
    state: Arc<Mutex<State>>,
}

impl DocGet {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for DocGet {
    fn name(&self) -> &str {
        "doc get"
    }

    fn signature(&self) -> Signature {
        Signature::build("doc get")
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
            .category(Category::Custom("couchbase".into()))
    }

    fn usage(&self) -> &str {
        "Fetches a document through the data service"
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

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Fetches a single document with the ID as an argument",
                example: "doc get my_doc_id",
                result: None,
            },
            Example {
                description: "Fetches multiple documents with IDs from the previous command",
                example: "echo [[id]; [airline_10] [airline_11]] | doc get",
                result: None,
            },
        ]
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

    let cluster_identifiers = cluster_identifiers_from(&engine_state, stack, &state, &call, true)?;
    let batch_size: Option<i64> = call.get_flag(&engine_state, stack, "batch-size")?;
    let id_column: String = call
        .get_flag(&engine_state, stack, "id-column")?
        .unwrap_or_else(|| "id".into());
    let ids = ids_from_input(&call, input, id_column.clone(), ctrl_c.clone())?;

    let mut workers = FuturesUnordered::new();
    let guard = state.lock().unwrap();

    let mut all_ids: Vec<Vec<String>> = vec![];
    if let Some(size) = batch_size {
        all_ids = build_batched_kv_items(size as u32, ids.clone());
    }

    let bucket_flag = call.get_flag(engine_state, stack, "bucket")?;
    let scope_flag = call.get_flag(engine_state, stack, "scope")?;
    let collection_flag = call.get_flag(engine_state, stack, "collection")?;

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

        if all_ids.is_empty() {
            all_ids = build_batched_kv_items(active_cluster.kv_batch_size(), ids.clone());
        }

        debug!("Running kv get for docs {:?}", &ids);

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

        let client = Arc::new(client);

        for ids in all_ids.clone() {
            for id in ids {
                let deadline = Instant::now().add(active_cluster.timeouts().data_timeout());

                let scope = scope.clone();
                let collection = collection.clone();
                let ctrl_c = ctrl_c.clone();
                let id = id.clone();

                let client = client.clone();

                workers.push(async move {
                    client
                        .request(
                            KeyValueRequest::Get { key: id },
                            scope,
                            collection,
                            deadline,
                            ctrl_c,
                        )
                        .await
                });
            }
            rt.block_on(async {
                while let Some(response) = workers.next().await {
                    match response {
                        Ok(mut res) => {
                            let mut collected = NuValueMap::default();
                            collected.add_string(&id_column, res.key(), call.head);
                            collected.add_i64("cas", res.cas() as i64, call.head);
                            let content = res.content().unwrap();
                            match convert_json_value_to_nu_value(&content, call.head) {
                                Ok(c) => {
                                    collected.add("content", c);
                                    collected.add_string("error", "", call.head);
                                }
                                Err(e) => {
                                    collected.add_string("content", "", call.head);
                                    collected.add_string("error", e.to_string(), call.head);
                                }
                            }
                            collected.add_string("cluster", identifier.clone(), call.head);
                            results.push(collected.into_value(call.head));
                        }
                        Err(e) => {
                            let mut collected = NuValueMap::default();
                            collected.add_string(
                                &id_column,
                                e.key().unwrap_or_else(|| "".to_string()),
                                call.head,
                            );
                            collected.add_string("cas", "", call.head);
                            collected.add_string("content", "", call.head);
                            collected.add_string("error", e.to_string(), call.head);
                            collected.add_string("cluster", identifier.clone(), call.head);
                            results.push(collected.into_value(call.head));
                        }
                    }
                }
            });
        }
    }

    Ok(Value::List {
        vals: results,
        span: call.head,
    }
    .into_pipeline_data())
}

pub(crate) fn ids_from_input(
    args: &Call,
    input: PipelineData,
    id_column: String,
    ctrl_c: Arc<AtomicBool>,
) -> Result<Vec<String>, ShellError> {
    let mut ids: Vec<String> = input
        .into_interruptible_iter(Some(ctrl_c))
        .map(move |v| match v {
            Value::String { val, .. } => Some(val.clone()),
            Value::Record { cols, vals, .. } => {
                if let Some(idx) = cols.iter().position(|x| x.clone() == id_column) {
                    if let Some(d) = vals.get(idx) {
                        match d {
                            Value::String { val, .. } => Some(val.clone()),
                            _ => None,
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        })
        .flatten()
        .collect();

    if let Some(id) = args.positional_nth(0) {
        if let Some(i) = id.as_string() {
            ids.push(i);
        }
    }

    Ok(ids)
}
