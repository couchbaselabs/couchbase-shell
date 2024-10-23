use crate::cli::error::{
    analytics_error, client_error_to_shell_error, deserialize_error, malformed_response_error,
    unexpected_status_code_error, AnalyticsErrorReason,
};
use crate::cli::util::{
    cluster_identifiers_from, convert_json_value_to_nu_value, convert_row_to_nu_value,
    duration_to_golang_string, get_active_cluster,
};
use crate::client::{AnalyticsQueryRequest, HttpResponse};
use crate::state::State;
use crate::RemoteCluster;
use log::debug;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, ListStream, PipelineData, ShellError, Signature, Span, SyntaxShape, Value,
};
use std::collections::HashMap;
use std::ops::Add;
use std::str::from_utf8;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;
use tokio::time::Instant;
use utilities::json_row_stream::JsonRowStream;
use utilities::raw_json_row_streamer::RawJsonRowStreamer;

#[derive(Clone)]
pub struct Analytics {
    state: Arc<Mutex<State>>,
}

impl Analytics {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for Analytics {
    fn name(&self) -> &str {
        "analytics"
    }

    fn signature(&self) -> Signature {
        Signature::build("analytics")
            .required("statement", SyntaxShape::String, "the analytics statement")
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
            .switch("with-meta", "Includes related metadata in the result", None)
            .named(
                "clusters",
                SyntaxShape::String,
                "the clusters which should be contacted",
                None,
            )
            .category(Category::Custom("couchbase".to_string()))
    }

    fn usage(&self) -> &str {
        "Performs an analytics query"
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

pub struct AnalyticsStream {
    span: Span,
    identifiers: Vec<String>,
    streams: HashMap<String, RawJsonRowStreamer>,
    rt: Arc<Runtime>,
}

impl Iterator for AnalyticsStream {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        if self.identifiers.is_empty() {
            return None;
        }

        let cluster = &self.identifiers[0].clone();
        let runtime = self.rt.clone();
        runtime.block_on(async {
            if let Some(next) = self
                .streams
                .get_mut(cluster)
                .unwrap()
                .read_row()
                .await
                .unwrap()
                .clone()
            {
                let result_string = from_utf8(&next).unwrap();
                let (start, _) = result_string.split_at(result_string.len() - 1);
                let with_cluster = format!("{}, \"cluster\": \"{}\" }}", start, cluster);
                let json_object = serde_json::from_str::<serde_json::Value>(&with_cluster).unwrap();
                Some(convert_json_value_to_nu_value(&json_object, self.span).unwrap())
            } else {
                self.identifiers.remove(0);
                self.next()
            }
        })
    }
}

fn run(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();
    let statement: String = call.req(engine_state, stack, 0)?;
    let span = call.head;

    let cluster_identifiers = cluster_identifiers_from(engine_state, stack, &state, call, true)?;

    let guard = state.lock().unwrap();

    let scope: Option<String> = call.get_flag(engine_state, stack, "scope")?;
    let _with_meta = call.has_flag(engine_state, stack, "with-meta")?;

    debug!("Running analytics query {}", &statement);

    // TO DO
    // Fix querying multiple clusters - currently panics due to runtime creation in utilities/src/lib.rs:69
    // Move Do/Send analytics queries to some utils file
    let mut streams = HashMap::new();
    let rt = Runtime::new().unwrap();
    for identifier in cluster_identifiers.clone() {
        let active_cluster = get_active_cluster(identifier.clone(), &guard, span)?;
        let bucket = call
            .get_flag(engine_state, stack, "bucket")?
            .or_else(|| active_cluster.active_bucket());
        let maybe_scope = bucket.and_then(|b| scope.clone().map(|s| (b, s)));

        let stream = rt.block_on(async {
            send_analytics_query_stream(
                active_cluster,
                maybe_scope,
                statement.clone(),
                ctrl_c.clone(),
                span,
            )
            .await
        })?;

        streams.insert(identifier.clone(), stream);
    }

    let result_stream = AnalyticsStream {
        span,
        streams,
        identifiers: cluster_identifiers,
        rt: Arc::new(rt),
    };

    Ok(PipelineData::from(ListStream::new(
        result_stream,
        span,
        None,
    )))
}

pub async fn send_analytics_query_stream(
    active_cluster: &RemoteCluster,
    scope: impl Into<Option<(String, String)>>,
    statement: impl Into<String>,
    ctrl_c: Arc<AtomicBool>,
    span: Span,
) -> Result<RawJsonRowStreamer, ShellError> {
    let (stream, status) = active_cluster
        .cluster()
        .http_client()
        .analytics_query_stream_request(
            AnalyticsQueryRequest::Execute {
                statement: statement.into(),
                scope: scope.into(),
                timeout: duration_to_golang_string(active_cluster.timeouts().analytics_timeout()),
            },
            Instant::now().add(active_cluster.timeouts().analytics_timeout()),
            ctrl_c.clone(),
        )
        .await
        .map_err(|e| client_error_to_shell_error(e, span))?;

    let json_stream = JsonRowStream::new(stream);
    let mut json_streamer = RawJsonRowStreamer::new(json_stream, "".to_string());

    // TO DO - less hacky reading of the first two lines
    // Read prelude and signature
    json_streamer.read_prelude().await.unwrap();
    json_streamer.read_row().await.unwrap();

    if status != 200 {
        let error_msg = if let Some(chunk) = json_streamer.read_row().await? {
            from_utf8(&chunk).unwrap().to_string()
        } else {
            "could not parse errors from stream".to_string()
        };

        return Err(unexpected_status_code_error(status, error_msg, span));
    }

    // Read row containing `results: [`
    json_streamer.read_row().await.unwrap();

    Ok(json_streamer)
}

pub fn send_analytics_query(
    active_cluster: &RemoteCluster,
    scope: impl Into<Option<(String, String)>>,
    statement: impl Into<String>,
    ctrl_c: Arc<AtomicBool>,
    span: Span,
) -> Result<HttpResponse, ShellError> {
    let response = active_cluster
        .cluster()
        .http_client()
        .analytics_query_request(
            AnalyticsQueryRequest::Execute {
                statement: statement.into(),
                scope: scope.into(),
                timeout: duration_to_golang_string(active_cluster.timeouts().analytics_timeout()),
            },
            Instant::now().add(active_cluster.timeouts().analytics_timeout()),
            ctrl_c,
        )
        .map_err(|e| client_error_to_shell_error(e, span))?;

    if response.status() != 200 {
        return Err(unexpected_status_code_error(
            response.status(),
            response.content(),
            span,
        ));
    }

    Ok(response)
}

pub fn do_analytics_query(
    identifier: String,
    active_cluster: &RemoteCluster,
    scope: impl Into<Option<(String, String)>>,
    statement: impl Into<String>,
    ctrl_c: Arc<AtomicBool>,
    span: Span,
    with_meta: bool,
    could_contain_mutations: bool,
) -> Result<Vec<Value>, ShellError> {
    let response = send_analytics_query(active_cluster, scope, statement, ctrl_c, span)?;

    let content: serde_json::Value = serde_json::from_str(response.content())
        .map_err(|e| deserialize_error(e.to_string(), span))?;

    let mut results: Vec<Value> = vec![];
    if with_meta {
        let converted = &mut convert_row_to_nu_value(&content, span, identifier)?;
        results.append(converted);
        return Ok(results);
    }

    if let Some(content_errors) = content.get("errors") {
        return if let Some(arr) = content_errors.as_array() {
            if arr.len() == 1 {
                let e = match arr.first() {
                    Some(e) => e,
                    None => {
                        return Err(malformed_response_error(
                            "analytics errors present but empty",
                            content_errors.to_string(),
                            span,
                        ))
                    }
                };
                let code = e.get("code").map(|c| c.as_i64().unwrap_or_default());
                let reason = match code {
                    Some(c) => AnalyticsErrorReason::from(c),
                    None => AnalyticsErrorReason::UnknownError,
                };
                let msg = match e.get("msg") {
                    Some(msg) => msg.to_string(),
                    None => "".to_string(),
                };
                Err(analytics_error(reason, code, msg, span))
            } else {
                let messages = arr
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<String>>()
                    .join(",");

                Err(analytics_error(
                    AnalyticsErrorReason::MultiErrors,
                    None,
                    messages,
                    span,
                ))
            }
        } else {
            Err(malformed_response_error(
                "analytics errors not an array",
                content_errors.to_string(),
                span,
            ))
        };
    } else if let Some(content_results) = content.get("results") {
        if let Some(arr) = content_results.as_array() {
            for result in arr {
                results.append(&mut convert_row_to_nu_value(
                    result,
                    span,
                    identifier.clone(),
                )?)
            }
        } else {
            return Err(malformed_response_error(
                "analytics rows not an array",
                content_results.to_string(),
                span,
            ));
        }
    } else if !could_contain_mutations {
        return Err(malformed_response_error(
            "analytics toplevel result not an object",
            content.to_string(),
            span,
        ));
    }

    Ok(results)
}
