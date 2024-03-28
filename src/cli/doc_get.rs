//! The `doc get` command performs a KV get operation.

use super::util::convert_json_value_to_nu_value;
use crate::state::State;

use crate::cli::doc_upsert::{build_batched_kv_items, get_active_cluster_client_cid};
use crate::cli::util::{cluster_identifiers_from, NuValueMap};
use crate::client::KeyValueRequest;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use log::debug;
use std::ops::Add;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;
use tokio::time::Instant;

use crate::cli::error::generic_error;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Span, SyntaxShape,
    Value,
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
                "databases",
                SyntaxShape::String,
                "the databases which should be contacted",
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

    let cluster_identifiers = cluster_identifiers_from(engine_state, stack, &state, call, true)?;
    let batch_size: Option<i64> = call.get_flag(engine_state, stack, "batch-size")?;
    let id_column: String = call
        .get_flag(engine_state, stack, "id-column")?
        .unwrap_or_else(|| "id".to_string());
    let ids = ids_from_input(
        input,
        id_column.clone(),
        ctrl_c.clone(),
        call.positional_nth(0),
    )?;

    let mut workers = FuturesUnordered::new();
    let guard = state.lock().unwrap();

    let mut all_ids: Vec<Vec<String>> = vec![];
    if let Some(size) = batch_size {
        all_ids = build_batched_kv_items(size as u32, ids.clone());
    }

    let bucket_flag = call.get_flag(engine_state, stack, "bucket")?;
    let scope_flag = call.get_flag(engine_state, stack, "scope")?;
    let collection_flag = call.get_flag(engine_state, stack, "collection")?;
    let halt_on_error = call.has_flag(engine_state, stack, "halt-on-error")?;

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
            ctrl_c.clone(),
            span,
        ) {
            Ok(c) => c,
            Err(e) => {
                if halt_on_error {
                    return Err(e);
                }

                let collected = GetResult::new(identifier.clone())
                    .id_column(&id_column)
                    .error(e.to_string())
                    .into_value(call.head);
                results.push(collected);
                continue;
            }
        };

        if all_ids.is_empty() {
            all_ids = build_batched_kv_items(active_cluster.kv_batch_size(), ids.clone());
        }

        debug!("Running kv get for docs {:?}", &ids);

        for ids in all_ids.clone() {
            for id in ids {
                let deadline = Instant::now().add(active_cluster.timeouts().data_timeout());

                let ctrl_c = ctrl_c.clone();
                let id = id.clone();

                let client = client.clone();

                workers.push(async move {
                    client
                        .request(KeyValueRequest::Get { key: id }, cid, deadline, ctrl_c)
                        .await
                });
            }
            rt.block_on(async {
                while let Some(response) = workers.next().await {
                    match response {
                        Ok(mut res) => {
                            let mut collected = GetResult::new(&identifier)
                                .id_column(&id_column)
                                .key(res.key())
                                .cas(res.cas() as i64);

                            let content = res.content().unwrap_or_default();
                            match convert_json_value_to_nu_value(&content, call.head) {
                                Ok(c) => {
                                    collected = collected.content(c);
                                }
                                Err(e) => {
                                    if halt_on_error {
                                        return Err(e);
                                    }
                                    collected = collected.error(e.to_string());
                                }
                            }
                            results.push(collected.into_value(call.head));
                        }
                        Err(e) => {
                            if halt_on_error {
                                return Err(generic_error(
                                    "Failed to fetch document",
                                    Some(e.to_string()),
                                    call.head,
                                ));
                            }

                            let collected = GetResult::new(&identifier)
                                .id_column(&id_column)
                                .key(e.key().unwrap_or_default())
                                .error(e.to_string())
                                .into_value(call.head);
                            results.push(collected);
                        }
                    }
                }
                Ok(())
            })?;
        }
    }

    Ok(Value::List {
        vals: results,
        internal_span: call.head,
    }
    .into_pipeline_data())
}

pub(crate) fn ids_from_input(
    input: PipelineData,
    id_column: String,
    ctrl_c: Arc<AtomicBool>,
    id: Option<&nu_protocol::ast::Expression>,
) -> Result<Vec<String>, ShellError> {
    let mut ids: Vec<String> = input
        .into_interruptible_iter(Some(ctrl_c))
        .filter_map(move |v| match v {
            Value::String { val, .. } => Some(val),
            Value::Record { val, .. } => {
                if let Some(d) = val.get(id_column.clone()) {
                    match d {
                        Value::String { val, .. } => Some(val.clone()),
                        _ => None,
                    }
                } else {
                    None
                }
            }
            _ => None,
        })
        .collect();

    if let Some(id) = id {
        if let Some(i) = id.as_string() {
            ids.push(i);
        }
    }

    Ok(ids)
}

#[derive(Debug)]
pub(crate) struct GetResult {
    error: Option<String>,
    content: Option<Value>,
    key: Option<String>,
    cluster: String,
    cas: Option<i64>,
    id_column: Option<String>,
}

impl GetResult {
    pub fn new(cluster: impl Into<String>) -> GetResult {
        Self {
            error: None,
            content: None,
            key: None,
            cluster: cluster.into(),
            cas: None,
            id_column: None,
        }
    }

    pub fn id_column(mut self, id_column: impl Into<String>) -> Self {
        self.id_column = Some(id_column.into());
        self
    }

    pub fn content(mut self, content: Value) -> GetResult {
        self.content = Some(content);
        self
    }

    pub fn key(mut self, key: String) -> GetResult {
        self.key = Some(key);
        self
    }

    pub fn cas(mut self, cas: i64) -> GetResult {
        self.cas = Some(cas);
        self
    }

    pub fn error(mut self, err: String) -> GetResult {
        self.error = Some(err);
        self
    }

    pub fn into_value(self, span: Span) -> Value {
        let mut collected = NuValueMap::default();
        collected.add_string(
            self.id_column.unwrap_or_else(|| "id".to_string()),
            self.key.unwrap_or_default(),
            span,
        );
        collected.add("content", self.content.unwrap_or_default());
        collected.add_i64("cas", self.cas.unwrap_or_default(), span);
        collected.add_string("error", self.error.unwrap_or_default(), span);
        collected.add_string("cluster", self.cluster, span);
        collected.into_value(span)
    }
}
