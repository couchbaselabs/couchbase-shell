use super::util::convert_json_value_to_nu_value;
use crate::state::State;

use crate::cli::doc_common::{build_batched_kv_items, get_active_cluster_client_cid};
use crate::cli::doc_get::ids_from_input;
use crate::cli::doc_get::GetResult;
use crate::cli::util::cluster_identifiers_from;
use crate::client::KeyValueRequest;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use log::debug;
use nu_protocol::Example;
use nu_protocol::Record;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;
use tokio::time::Instant;

use crate::cli::error::generic_error;
use nu_engine::command_prelude::Call;
use nu_engine::CallExt;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct SubDocGet {
    state: Arc<Mutex<State>>,
}

impl SubDocGet {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for SubDocGet {
    fn name(&self) -> &str {
        "subdoc get"
    }

    fn signature(&self) -> Signature {
        Signature::build("subdoc get")
            .required(
                "path",
                SyntaxShape::Any,
                "the path(s) to be fetched from the documents",
            )
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
        "Fetches the value of the provided path in the specified document through the data service"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        run_subdoc_lookup(self.state.clone(), engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example{
                description: "Fetches the address and content fields from the document with the ID landmark_10019",
                example: "subdoc get [address content] landmark_10019",
                result: None
            },
            Example{
                description: "Fetches address field from multiple documents with IDs from the previous command",
                example: "[landmark_10019 landmark_10020] | subdoc get address",
                result: None
            },
        ]
    }

    fn requires_ast_for_arguments(&self) -> bool {
        true
    }
}

fn run_subdoc_lookup(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let signals = engine_state.signals().clone();

    let cluster_identifiers = cluster_identifiers_from(engine_state, stack, &state, call, true)?;

    let paths: Vec<String> = match call.req::<Value>(engine_state, stack, 0)? {
        Value::String { val, .. } => {
            vec![val]
        }
        Value::List { vals, .. } => vals
            .iter()
            .map(|s| s.as_str().unwrap().to_string())
            .collect(),
        _ => {
            return Err(generic_error(
                "Field(s) must be a string or list",
                "Run 'subdoc get --help' to see examples".to_string(),
                None,
            ));
        }
    };

    let id_column: String = call
        .get_flag(engine_state, stack, "id-column")?
        .unwrap_or_else(|| "id".to_string());
    let ids = ids_from_input(input, id_column.clone(), call.positional_nth(stack, 1))?;

    let mut workers = FuturesUnordered::new();
    let guard = state.lock().unwrap();

    let mut all_ids: Vec<Vec<String>> = vec![];

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
            signals.clone(),
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

        debug!(
            "Running kv subdoc multi lookup for docs {:?} on field ",
            &ids
        );

        for ids in all_ids.clone() {
            for id in ids {
                let deadline = Instant::now().add(active_cluster.timeouts().data_timeout());

                let signals = signals.clone();
                let id = id.clone();

                let client = client.clone();
                let copy = paths.clone();

                let request = if paths.len() > 1 {
                    KeyValueRequest::SubdocMultiLookup {
                        key: id,
                        paths: copy,
                    }
                } else {
                    KeyValueRequest::SubDocGet {
                        key: id.clone(),
                        path: paths[0].clone(),
                    }
                };

                workers.push(async move { client.request(request, cid, deadline, signals).await });
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

                            // Create a record where cols =  field and
                            match convert_json_value_to_nu_value(&content, call.head) {
                                Ok(c) => {
                                    if paths.len() == 1 {
                                        collected = collected.content(c);
                                    } else {
                                        let list = c.as_list().unwrap().to_vec();

                                        let record = Value::record(
                                            Record::from_raw_cols_vals(
                                                paths.clone(),
                                                list,
                                                span,
                                                span,
                                            )
                                            .unwrap(),
                                            span,
                                        );
                                        collected = collected.content(record);
                                    }
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

    Ok(Value::list(results, call.head).into_pipeline_data())
}
