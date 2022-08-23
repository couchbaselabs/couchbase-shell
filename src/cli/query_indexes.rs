use crate::cli::error::{
    deserialize_error, malformed_response_error, unexpected_status_code_error,
};
use crate::cli::util::{
    cluster_identifiers_from, convert_row_to_nu_value, duration_to_golang_string,
    get_active_cluster, NuValueMap,
};
use crate::client::{ManagementRequest, QueryRequest};
use crate::state::{RemoteCluster, State};
use log::debug;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};
use serde::Deserialize;
use std::ops::Add;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

#[derive(Clone)]
pub struct QueryIndexes {
    state: Arc<Mutex<State>>,
}

impl QueryIndexes {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for QueryIndexes {
    fn name(&self) -> &str {
        "query indexes"
    }

    fn signature(&self) -> Signature {
        Signature::build("query indexes")
            .switch(
                "definitions",
                "Whether to fetch the index definitions (changes the output format)",
                None,
            )
            .named(
                "clusters",
                SyntaxShape::String,
                "the clusters to query against",
                None,
            )
            .switch("with-meta", "Includes related metadata in the result", None)
            .category(Category::Custom("couchbase".to_string()))
    }

    fn usage(&self) -> &str {
        "Lists all query indexes"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        query(self.state.clone(), engine_state, stack, call, input)
    }
}

fn query(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();
    let with_meta = call.has_flag("with-meta");

    let cluster_identifiers = cluster_identifiers_from(engine_state, stack, &state, call, true)?;
    let guard = state.lock().unwrap();

    let fetch_defs = call.has_flag("definitions");

    let statement = "select keyspace_id as `bucket`, name, state, `using` as `type`, ifmissing(condition, null) as condition, ifmissing(is_primary, false) as `primary`, index_key from system:indexes";

    debug!("Running n1ql query {}", &statement);

    let mut results: Vec<Value> = vec![];
    for identifier in cluster_identifiers {
        let active_cluster = get_active_cluster(identifier.clone(), &guard, span)?;

        if fetch_defs {
            let mut defs =
                index_definitions(active_cluster, ctrl_c.clone(), identifier.clone(), span)?;
            results.append(&mut defs);
            continue;
        }

        let response = active_cluster.cluster().http_client().query_request(
            QueryRequest::Execute {
                statement: statement.to_string(),
                scope: None,
                timeout: duration_to_golang_string(active_cluster.timeouts().query_timeout()),
            },
            Instant::now().add(active_cluster.timeouts().query_timeout()),
            ctrl_c.clone(),
        )?;

        let content: serde_json::Value =
            serde_json::from_str(response.content()).map_err(|_e| {
                unexpected_status_code_error(response.status(), response.content(), span)
            })?;
        if with_meta {
            let converted = convert_row_to_nu_value(&content, span, identifier.clone())?;
            results.push(converted);
        } else if let Some(content_results) = content.get("results") {
            if let Some(arr) = content_results.as_array() {
                for result in arr {
                    results.push(convert_row_to_nu_value(result, span, identifier.clone())?);
                }
            } else {
                return Err(malformed_response_error(
                    "query results not an array",
                    content_results.to_string(),
                    span,
                ));
            }
        } else {
            return Err(malformed_response_error(
                "query top level object not an object",
                content.to_string(),
                span,
            ));
        }
    }

    Ok(Value::List {
        vals: results,
        span: call.head,
    }
    .into_pipeline_data())
}

#[derive(Debug, Deserialize)]
struct IndexDefinition {
    bucket: String,
    definition: String,
    collection: Option<String>,
    scope: Option<String>,
    #[serde(rename = "indexName")]
    index_name: String,
    status: String,
    #[serde(rename = "storageMode")]
    storage_mode: String,
    #[serde(rename = "numReplica")]
    replicas: u8,
}

#[derive(Debug, Deserialize)]
struct IndexStatus {
    indexes: Vec<IndexDefinition>,
}

fn index_definitions(
    cluster: &RemoteCluster,
    ctrl_c: Arc<AtomicBool>,
    identifier: String,
    span: Span,
) -> Result<Vec<Value>, ShellError> {
    debug!("Running fetch n1ql indexes");

    let response = cluster.cluster().http_client().management_request(
        ManagementRequest::IndexStatus,
        Instant::now().add(cluster.timeouts().query_timeout()),
        ctrl_c,
    )?;

    match response.status() {
        200 => {}
        _ => {
            return Err(unexpected_status_code_error(
                response.status(),
                response.content(),
                span,
            ));
        }
    }

    let defs: IndexStatus = serde_json::from_str(response.content())
        .map_err(|e| deserialize_error(e.to_string(), span))?;
    let n = defs
        .indexes
        .into_iter()
        .map(|d| {
            let mut collected = NuValueMap::default();
            collected.add_string("bucket", d.bucket, span);
            collected.add_string("scope", d.scope.unwrap_or_else(|| "".to_string()), span);
            collected.add_string(
                "collection",
                d.collection.unwrap_or_else(|| "".to_string()),
                span,
            );
            collected.add_string("name", d.index_name, span);
            collected.add_string("status", d.status, span);
            collected.add_string("storage_mode", d.storage_mode, span);
            collected.add_i64("replicas", d.replicas as i64, span);
            collected.add_string("definition", d.definition, span);
            collected.add_string("cluster", identifier.clone(), span);

            collected.into_value(span)
        })
        .collect::<Vec<_>>();

    Ok(n)
}
