use crate::cli::error::{deserialize_error, unexpected_status_code_error};
use crate::cli::query::{handle_query_response, query_context_from_args, send_query};
use crate::cli::util::{cluster_identifiers_from, get_active_cluster, NuValueMap};
use crate::client::ManagementRequest;
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
            .switch("disable-context", "disable automatically detecting the query context based on the active bucket and scope", None)
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

    let cluster_identifiers = cluster_identifiers_from(engine_state, stack, &state, call, true)?;

    let fetch_defs = call.has_flag("definitions");

    let statement = "select bucket_id as `bucket`, scope_id as `scope`, keyspace_id as `keyspace`, name, state, `using` as `type`, \
    ifmissing(condition, null) as condition, ifmissing(is_primary, false) as `primary`, \
    index_key, CASE metadata.stats.last_known_scan_time WHEN 0 THEN 0 ELSE \
    MILLIS_TO_STR(TRUNC(metadata.stats.last_known_scan_time / 1000000, 0)) END as `last_known_scan_time` from system:indexes"
        .to_string();

    debug!("Running n1ql query {}", &statement);

    let mut results: Vec<Value> = vec![];
    for identifier in cluster_identifiers {
        let guard = state.lock().unwrap();
        let active_cluster = get_active_cluster(identifier.clone(), &guard, span)?;
        let maybe_scope = query_context_from_args(active_cluster, engine_state, stack, call)?;

        if fetch_defs {
            let mut defs =
                index_definitions(active_cluster, ctrl_c.clone(), identifier.clone(), span)?;
            results.append(&mut defs);
            continue;
        }

        let response = send_query(
            active_cluster,
            statement.clone(),
            maybe_scope,
            ctrl_c.clone(),
        )?;
        drop(guard);

        results.extend(handle_query_response(
            call.has_flag("with-meta"),
            identifier.clone(),
            response,
            span,
        )?);
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
