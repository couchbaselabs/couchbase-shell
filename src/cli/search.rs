use crate::cli::error::unexpected_status_code_error;
use crate::cli::util::{
    cluster_identifiers_from, duration_to_golang_string, get_active_cluster, NuValueMap,
};
use crate::client::SearchQueryRequest;
use crate::state::State;
use log::debug;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};
use serde_derive::Deserialize;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

#[derive(Clone)]
pub struct Search {
    state: Arc<Mutex<State>>,
}

impl Search {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for Search {
    fn name(&self) -> &str {
        "search"
    }

    fn signature(&self) -> Signature {
        Signature::build("search")
            .required("index", SyntaxShape::String, "the index name")
            .required(
                "query",
                SyntaxShape::String,
                "the text to query for using a query string query",
            )
            .named(
                "clusters",
                SyntaxShape::String,
                "the clusters which should be contacted",
                None,
            )
            .category(Category::Custom("couchbase".into()))
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
    let query: String = call.req(engine_state, stack, 1)?;

    debug!("Running search query {} against {}", &query, &index);

    let cluster_identifiers = cluster_identifiers_from(&engine_state, stack, &state, &call, true)?;
    let guard = state.lock().unwrap();

    let mut results = vec![];
    for identifier in cluster_identifiers {
        let active_cluster = get_active_cluster(identifier.clone(), &guard, span.clone())?;
        let response = active_cluster
            .cluster()
            .http_client()
            .search_query_request(
                SearchQueryRequest::Execute {
                    query: query.clone(),
                    index: index.clone(),
                    timeout: duration_to_golang_string(active_cluster.timeouts().search_timeout()),
                },
                Instant::now().add(active_cluster.timeouts().search_timeout()),
                ctrl_c.clone(),
            )?;

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
