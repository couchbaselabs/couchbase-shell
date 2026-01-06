use crate::cli::analytics_common::send_analytics_query;
use crate::cli::util::{
    cluster_identifiers_from, convert_json_value_to_nu_value, get_active_cluster,
};
use crate::state::State;
use futures::StreamExt;
use log::debug;
use nu_engine::command_prelude::Call;
use nu_engine::CallExt;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, ListStream, PipelineData, ShellError, Signature, Span, SyntaxShape,
    Value,
};
use std::str::from_utf8;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;
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
                    return Some(Value::error(e, self.span));
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
    let with_meta = call.has_flag(engine_state, stack, "with-meta")?;

    debug!("Running analytics query {}", &statement);

    let mut results: Vec<Value> = vec![];
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

        if with_meta {
            let mut query_results = vec![];
            while let Some(result_row) = rt
                .block_on(async { json_streamer.read_row().await })
                .map_err(|e| ShellError::GenericError {
                    error: format!("failed to read analytics query result: {}", e),
                    msg: "".to_string(),
                    span: None,
                    help: None,
                    inner: vec![],
                })?
            {
                let row_str = from_utf8(&result_row).unwrap();
                let row_json = serde_json::from_str::<serde_json::Value>(row_str).unwrap();
                query_results.push(convert_json_value_to_nu_value(&row_json, span).unwrap())
            }

            let meta = rt
                .block_on(async { json_streamer.read_epilog().await })
                .map_err(|e| ShellError::GenericError {
                    error: format!("failed to read stream epilog: {}", e),
                    msg: "".to_string(),
                    span: None,
                    help: None,
                    inner: vec![],
                })?;
            let meta_json =
                serde_json::from_str::<serde_json::Value>(from_utf8(&meta).unwrap()).unwrap();
            let meta_value = convert_json_value_to_nu_value(&meta_json, span).unwrap();
            let meta_as_record: &mut nu_protocol::Record = &mut meta_value.into_record().unwrap();

            meta_as_record.push("cluster", Value::string(identifier.clone(), span));

            meta_as_record.push("results", Value::list(query_results, span));

            results.push(Value::record(meta_as_record.clone(), span))
        }

        streams.insert(identifier, json_streamer);
    }

    let result_stream = AnalyticsStream { streams, span, rt };

    if with_meta {
        return Ok(Value::list(results, span).into_pipeline_data());
    }

    Ok(PipelineData::from(ListStream::new(
        result_stream,
        span,
        signals,
    )))
}
