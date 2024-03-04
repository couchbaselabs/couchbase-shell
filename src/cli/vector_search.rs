use crate::cli::error::{client_error_to_shell_error, unexpected_status_code_error};
use crate::cli::util::{cluster_identifiers_from, get_active_cluster, NuValueMap};
use crate::client::VectorSearchQueryRequest;
use crate::state::State;
use log::debug;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};
use serde_derive::Deserialize;
use serde_json::json;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

#[derive(Clone)]
pub struct VectorSearch {
    state: Arc<Mutex<State>>,
}

impl VectorSearch {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for VectorSearch {
    fn name(&self) -> &str {
        "vector search"
    }

    fn signature(&self) -> Signature {
        Signature::build("vector search")
            .required("index", SyntaxShape::String, "the index name")
            .required(
                "vector",
                SyntaxShape::Any,
                "search vector to be queried for",
            )
            .required(
                "field",
                SyntaxShape::String,
                "name of the vector field the index was built on",
            )
            .named(
                "query",
                SyntaxShape::String,
                "the text to query for using a query string query",
                None,
            )
            .named(
                "databases",
                SyntaxShape::String,
                "the databases which should be contacted",
                None,
            )
            .named(
                "neighbours",
                SyntaxShape::Int,
                "number of neighbours returned by vector search (default = 3)",
                None,
            )
            .category(Category::Custom("couchbase".to_string()))
    }

    fn usage(&self) -> &str {
        "Performs a search query"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        run(self.state.clone(), engine_state, stack, call, input)
    }
}

fn run(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();

    let index: String = call.req(engine_state, stack, 0)?;
    let vector: Vec<f32> = call
        .req::<Vec<Value>>(engine_state, stack, 1)?
        .clone()
        .iter()
        .map(|e| e.as_float().unwrap() as f32)
        .collect();
    let field: String = call.req(engine_state, stack, 2)?;

    let query: serde_json::Value = match call.get_flag::<String>(engine_state, stack, "query")? {
        Some(q) => json!({ "query": q }),
        None => {
            json!({"match_none": {}})
        }
    };

    let neighbours: i64 = match call.get_flag(engine_state, stack, "neighbours")? {
        Some(n) => n,
        None => 3,
    };

    debug!("Running vector search query {} against {}", &query, &index);

    let cluster_identifiers = cluster_identifiers_from(engine_state, stack, &state, call, true)?;
    let guard = state.lock().unwrap();

    let mut results = vec![];
    for identifier in cluster_identifiers {
        let active_cluster = get_active_cluster(identifier.clone(), &guard, span)?;
        let response = active_cluster
            .cluster()
            .http_client()
            .search_query_request(
                VectorSearchQueryRequest::Execute {
                    query: query.clone(),
                    index: index.clone(),
                    vector: vector.clone(),
                    field: field.clone(),
                    neighbours: neighbours.clone(),
                    timeout: active_cluster.timeouts().search_timeout().as_millis(),
                },
                Instant::now().add(active_cluster.timeouts().search_timeout()),
                ctrl_c.clone(),
            )
            .map_err(|e| client_error_to_shell_error(e, span))?;

        let rows: SearchResultData = match response.status() {
            200 => serde_json::from_str(response.content()).map_err(|_e| {
                unexpected_status_code_error(response.status(), response.content(), span)
            })?,
            _ => {
                return Err(unexpected_status_code_error(
                    response.status(),
                    response.content(),
                    span,
                ));
            }
        };

        for row in rows.hits {
            let mut collected = NuValueMap::default();
            collected.add_string("id", row.id, span);
            collected.add_string("score", format!("{}", row.score), span);
            collected.add_string("index", row.index, span);
            collected.add_string("cluster", identifier.clone(), span);

            results.push(collected.into_value(span));
        }
    }

    Ok(Value::List {
        vals: results,
        span: call.head,
    }
    .into_pipeline_data())
}

#[derive(Debug, Deserialize)]
struct SearchResultHit {
    score: f32,
    index: String,
    id: String,
}

#[derive(Debug, Deserialize)]
struct SearchResultData {
    hits: Vec<SearchResultHit>,
}
