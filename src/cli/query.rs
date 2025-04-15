use crate::cli::util::{
    cluster_identifiers_from, convert_row_to_nu_value, get_active_cluster, NuValueMap,
};
use crate::cli::util::{convert_json_value_to_nu_value, convert_nu_value_to_json_value};
use crate::state::State;
use log::debug;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::Instant;

use crate::cli::error::{client_error_to_shell_error, deserialize_error, malformed_response_error};
use crate::cli::generic_error;
use crate::client::connection_client::{HttpStreamResponse, QueryRequest};
use crate::client::query_metadata::QueryMetaData;
use crate::client::QueryTransactionRequest;
use crate::RemoteCluster;
use nu_engine::command_prelude::Call;
use nu_engine::CallExt;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::Value::Nothing;
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signals, Signature, Span,
    SyntaxShape, Value,
};
use serde_json::json;
use tokio::runtime::Runtime;

#[derive(Clone)]
pub struct Query {
    state: Arc<Mutex<State>>,
}

impl Query {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for Query {
    fn name(&self) -> &str {
        "query"
    }

    fn signature(&self) -> Signature {
        Signature::build("query")
            .required("statement", SyntaxShape::String, "the query statement")
            .named(
                "clusters",
                SyntaxShape::String,
                "the clusters to query against",
                None,
            )
            .named(
                "bucket",
                SyntaxShape::String,
                "the bucket to query against",
                None,
            )
            .named(
                "scope",
                SyntaxShape::String,
                "the scope to query against",
                None,
            )
            .named(
                "params",
                SyntaxShape::Any,
                "named or positional parameters for the query",
                None,
            )
            .switch("with-meta", "include toplevel metadata", None)
            .switch("disable-context", "disable automatically detecting the query context based on the active bucket and scope", None)
            .category(Category::Custom("couchbase".to_string()))
    }

    fn description(&self) -> &str {
        "Performs a n1ql query"
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

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Run a basic query",
                example: " query \"SELECT * FROM `travel-sample` WHERE type = 'landmark'\"",
                result: None,
            },
            Example {
                description:  "Pass query parameters as an object",
                example: "query \"SELECT airline FROM `travel-sample`.inventory.route WHERE sourceairport = $aval AND distance > $dval\" --params {aval: LAX dval: 13000}",
                result: None,
            },
            Example {
                description:  "Pass query parameters as a list",
                example: "query \"SELECT airline FROM `travel-sample`.inventory.route WHERE sourceairport = $1 AND distance > $2\" --params [LAX 13000]",
                result: None,
            }
        ]
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
    let signals = engine_state.signals().clone();

    let cluster_identifiers = cluster_identifiers_from(engine_state, stack, &state, call, true)?;

    let statement: String = call.req(engine_state, stack, 0)?;

    let params: Option<serde_json::Value> =
        match call.get_flag::<Value>(engine_state, stack, "params")? {
            Some(p) => match p {
                Value::Record { .. } => Some(convert_nu_value_to_json_value(&p, span)?),
                Value::List { .. } => Some(convert_nu_value_to_json_value(&p, span)?),
                _ => {
                    return Err(generic_error(
                        "Parameters must be a list or JSON object",
                        "Run 'query --help' to see examples".to_string(),
                        None,
                    ));
                }
            },
            None => None,
        };

    let mut results: Vec<Value> = vec![];
    let rt = Runtime::new()?;
    for identifier in cluster_identifiers {
        let guard = state.lock().unwrap();
        let active_cluster = get_active_cluster(identifier.clone(), &guard, span)?;

        let maybe_scope = query_context_from_args(active_cluster, engine_state, stack, call)?;

        debug!("Running n1ql query {}", &statement);

        let result = rt.block_on(async {
            let mut response = send_query(
                active_cluster,
                statement.clone(),
                params.clone(),
                maybe_scope,
                signals.clone(),
                None,
                span,
                None,
            )
            .await?;

            let contents = response
                .content()
                .await
                .map_err(|e| client_error_to_shell_error(e, span))?;

            let meta = response
                .metadata()
                .map_err(|e| client_error_to_shell_error(e, span))?
                .map(|m| m.query().cloned())
                .flatten();

            handle_query_response(
                call.has_flag(engine_state, stack, "with-meta")?,
                identifier.clone(),
                contents,
                meta,
                span,
            )
            .await
        })?;
        drop(guard);

        results.extend(result);
    }

    if !results.is_empty() {
        return Ok(Value::List {
            vals: results,
            internal_span: call.head,
        }
        .into_pipeline_data());
    }

    Ok(PipelineData::Value(
        Nothing {
            internal_span: span,
        },
        None,
    ))
}

pub async fn send_query(
    cluster: &RemoteCluster,
    statement: impl Into<String>,
    parameters: Option<serde_json::Value>,
    scope: Option<(String, String)>,
    signals: Signals,
    timeout: impl Into<Option<Duration>>,
    span: Span,
    transaction: impl Into<Option<QueryTransactionRequest>>,
) -> Result<HttpStreamResponse, ShellError> {
    let timeout = timeout.into().unwrap_or(cluster.timeouts().query_timeout());
    let statement = statement.into();
    let client = cluster
        .cluster()
        .connection_client(None, Instant::now().add(timeout), signals.clone())
        .await
        .map_err(|e| client_error_to_shell_error(e, span))?;

    let response = client
        .query(
            QueryRequest {
                statement: &statement,
                parameters,
                scope,
                transaction: transaction.into(),
                timeout,
            },
            Instant::now().add(timeout),
            signals,
        )
        .await
        .map_err(|e| client_error_to_shell_error(e, span))?;

    Ok(response)
}

pub async fn handle_query_response(
    with_meta: bool,
    identifier: String,
    content: Vec<Vec<u8>>,
    meta: Option<QueryMetaData>,
    span: Span,
) -> Result<Vec<Value>, ShellError> {
    let mut results: Vec<Value> = vec![];

    for row_content in content {
        let row: serde_json::Value = serde_json::from_slice(&row_content)
            .map_err(|e| deserialize_error(e.to_string(), span))?;

        results.append(&mut convert_row_to_nu_value(
            &row,
            span,
            identifier.clone(),
        )?);
    }

    if !with_meta {
        return Ok(results);
    }

    let mut content = NuValueMap::default();
    content.add_vec("results", results, span);

    if let Some(meta) = meta {
        let meta = json!(meta);
        if let Some(meta_obj) = meta.as_object() {
            for (key, value) in meta_obj {
                content.add(key, convert_json_value_to_nu_value(&value.clone(), span)?);
            }
        } else {
            // This shouldn't ever happen...
            return Err(malformed_response_error(
                "query metadata malformed",
                meta.to_string(),
                span,
            ));
        }
    } else {
        // This shouldn't ever happen...
        return Err(malformed_response_error(
            "query metadata missing",
            "".to_string(),
            span,
        ));
    }

    Ok(vec![content.into_value(span)])
}

pub fn query_context_from_args(
    cluster: &RemoteCluster,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<Option<(String, String)>, ShellError> {
    let bucket = call
        .get_flag(engine_state, stack, "bucket")?
        .or_else(|| cluster.active_bucket());

    let scope = call
        .get_flag(engine_state, stack, "scope")?
        .or_else(|| cluster.active_scope());

    let disable_context = call.has_flag(engine_state, stack, "disable-context")?;

    Ok(if disable_context {
        None
    } else {
        bucket.and_then(|b| scope.map(|s| (b, s)))
    })
}
