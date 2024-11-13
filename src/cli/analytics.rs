use crate::cli::error::{
    analytics_error, client_error_to_shell_error, deserialize_error, malformed_response_error,
    unexpected_status_code_error, AnalyticsErrorReason,
};
use crate::cli::util::{
    cluster_identifiers_from, convert_json_value_to_nu_value, convert_row_to_nu_value,
    duration_to_golang_string, get_active_cluster,
};
use crate::client::http_handler::HttpStreamResponse;
use crate::client::AnalyticsQueryRequest;
use crate::state::State;
use crate::RemoteCluster;
use futures::StreamExt;
use log::debug;
use nu_engine::command_prelude::Call;
use nu_engine::CallExt;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, ListStream, PipelineData, ShellError, Signals, Signature, Span, SyntaxShape, Value,
};
use std::ops::Add;
use std::str::from_utf8;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;
use tokio::time::Instant;
use tokio_stream::StreamMap;
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

    fn description(&self) -> &str {
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
    streams: StreamMap<String, RawJsonRowStreamer>,
    // This allows us to extend the lifetime of the runtime used to create the streams longer than
    // run(), else we panic when reading the streams
    rt: Arc<Runtime>,
}

impl Iterator for AnalyticsStream {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((cluster, result)) = self
            .rt
            .clone()
            .block_on(async { self.streams.next().await })
        {
            let bytes = match result {
                Ok(r) => r,
                Err(e) => {
                    return Some(Value::Error {
                        error: Box::new(e),
                        internal_span: self.span,
                    });
                }
            };
            let result_string = from_utf8(&bytes).unwrap();
            let (start, _) = result_string.split_at(result_string.len() - 1);
            let with_cluster = format!("{}, \"cluster\": \"{}\" }}", start, cluster);
            let json_object = serde_json::from_str::<serde_json::Value>(&with_cluster).unwrap();
            Some(convert_json_value_to_nu_value(&json_object, self.span).unwrap())
        } else {
            None
        }
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

    let cluster_identifiers = cluster_identifiers_from(engine_state, stack, &state, call, true)?;

    let guard = state.lock().unwrap();

    let signals = engine_state.signals().clone();
    let statement: String = call.req(engine_state, stack, 0)?;

    let scope: Option<String> = call.get_flag(engine_state, stack, "scope")?;

    debug!("Running analytics query {}", &statement);

    let mut streams = StreamMap::new();
    let rt = Arc::new(Runtime::new().unwrap());
    for identifier in cluster_identifiers.clone() {
        let active_cluster = get_active_cluster(identifier.clone(), &guard, span)?;
        let bucket = call
            .get_flag(engine_state, stack, "bucket")?
            .or_else(|| active_cluster.active_bucket());
        let maybe_scope = bucket.and_then(|b| scope.clone().map(|s| (b, s)));

        let resp = send_analytics_query(
            active_cluster,
            maybe_scope,
            statement.clone(),
            signals.clone(),
            span,
            rt.clone(),
        )?;

        let json_stream = JsonRowStream::new(resp.stream());
        let mut json_streamer = RawJsonRowStreamer::new(json_stream, "results");

        rt.block_on(async {
            // Read prelude so rows are ready for reading
            json_streamer.read_prelude().await
        })
        .map_err(|e| ShellError::GenericError {
            error: format!("failed to read stream prelude: {}", e),
            msg: "".to_string(),
            span: None,
            help: None,
            inner: vec![],
        })?;

        streams.insert(identifier, json_streamer);
    }

    let result_stream = AnalyticsStream { streams, span, rt };

    Ok(PipelineData::from(ListStream::new(
        result_stream,
        span,
        signals,
    )))
}

pub fn send_analytics_query(
    active_cluster: &RemoteCluster,
    scope: impl Into<Option<(String, String)>>,
    statement: impl Into<String>,
    signals: Signals,
    span: Span,
    rt: Arc<Runtime>,
) -> Result<HttpStreamResponse, ShellError> {
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
            signals.clone(),
            rt.clone(),
        )
        .map_err(|e| client_error_to_shell_error(e, span))?;

    if response.status() != 200 {
        return Err(unexpected_status_code_error(
            response.status(),
            response.content()?,
            span,
        ));
    }

    Ok(response)
}

pub fn read_analytics_response(
    identifier: String,
    response: HttpStreamResponse,
    span: Span,
    with_meta: bool,
    could_contain_mutations: bool,
) -> Result<Vec<Value>, ShellError> {
    let content = response.content()?;

    let content: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| deserialize_error(e.to_string(), span))?;

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
