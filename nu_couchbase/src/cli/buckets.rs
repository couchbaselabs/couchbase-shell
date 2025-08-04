use crate::cli::buckets_builder::{BucketSettings, JSONBucketSettings};
use crate::cli::buckets_get::bucket_to_nu_value;
use crate::cli::util::{cluster_identifiers_from, cluster_identifiers_from_plugin, get_active_cluster};
use crate::client::ManagementRequest;
use crate::state::State;
use log::debug;
use std::convert::TryFrom;
use std::ops::Add;
use std::sync::{Arc, Mutex, MutexGuard};
use tokio::time::Instant;

use crate::cli::error::{
    client_error_to_shell_error, deserialize_error, malformed_response_error,
    unexpected_status_code_error,
};
use crate::remote_cluster::RemoteCluster;
use nu_engine::command_prelude::Call;
use nu_plugin::{PluginCommand, SimplePluginCommand};
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, Example, IntoPipelineData, LabeledError, PipelineData, ShellError, Signals, Signature, Span, SyntaxShape, Value};
use crate::plugin::CouchbasePlugin;

#[derive(Clone)]
pub struct Buckets {
    state: Arc<Mutex<State>>,
}

impl Buckets {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for Buckets {
    fn name(&self) -> &str {
        _name()
    }

    fn signature(&self) -> Signature {
        _signature()
    }

    fn description(&self) -> &str {
        _description()
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let state = self.state.clone();
        let cluster_identifiers = cluster_identifiers_from(engine_state, stack, &state, call, true)?;

        let span = call.head;
        let signals = engine_state.signals().clone();

        let guard = state.lock().unwrap();

        let results = _get_all_buckets(cluster_identifiers, span, signals, &guard)?;

        Ok(Value::List {
            vals: results,
            internal_span: span,
        }
            .into_pipeline_data())
    }
}

impl PluginCommand for Buckets {
    type Plugin = CouchbasePlugin;

    fn name(&self) -> &str {
        _name()
    }

    fn signature(&self) -> Signature {
        _signature()
    }

    fn description(&self) -> &str {
        _description()
    }

    fn run(
        &self,
        plugin: &CouchbasePlugin,
        _engine: &nu_plugin::EngineInterface,
        call: &nu_plugin::EvaluatedCall,
        _input: PipelineData,
    ) -> Result<PipelineData, nu_protocol::LabeledError> {
        let state = plugin.state.clone();
        let cluster_identifiers = cluster_identifiers_from_plugin(
            plugin,
            _engine,
            call,
            _input, true)?;

        let span = call.head;
        let signals = _engine.signals().clone();

        let guard = state.lock().unwrap();

        let results = _get_all_buckets(cluster_identifiers, span, signals, &guard)?;

        Ok(Value::List {
            vals: results,
            internal_span: span,
        }
            .into_pipeline_data())
    }
}


fn _name() -> &'static str {
    "buckets"
}

fn _signature() -> Signature {
    Signature::build("buckets")
        .named(
            "clusters",
            SyntaxShape::String,
            "the clusters which should be contacted",
            None,
        )
        .category(Category::Custom("couchbase".to_string()))
}

fn _description() -> &'static str {
    "Perform bucket management operations"
}

fn buckets_get_all(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let cluster_identifiers = cluster_identifiers_from(engine_state, stack, &state, call, true)?;

    let span = call.head;
    let signals = engine_state.signals().clone();

    let guard = state.lock().unwrap();

    let results = _get_all_buckets(cluster_identifiers, span, signals, &guard)?;

    Ok(Value::List {
        vals: results,
        internal_span: span,
    }
    .into_pipeline_data())
}

fn _get_all_buckets(cluster_identifiers: Vec<String>, span: Span, signals: Signals, guard: &MutexGuard<State>) -> Result<Vec<Value>, ShellError> {
    debug!("Running buckets");

    let mut results = vec![];
    for identifier in cluster_identifiers {
        let cluster = get_active_cluster(identifier.clone(), &guard, span)?;

        for bucket in get_buckets(cluster, signals.clone(), span)? {
            results.push(bucket_to_nu_value(
                bucket,
                identifier.clone(),
                cluster.is_capella(),
                span,
            ));
        }
    }
    Ok(results)
}

pub fn get_buckets(
    cluster: &RemoteCluster,
    signals: Signals,
    span: Span,
) -> Result<Vec<BucketSettings>, ShellError> {
    let response = cluster
        .cluster()
        .http_client()
        .management_request(
            ManagementRequest::GetBuckets,
            Instant::now().add(cluster.timeouts().management_timeout()),
            signals.clone(),
        )
        .map_err(|e| client_error_to_shell_error(e, span))?;

    if response.status() != 200 {
        return Err(unexpected_status_code_error(
            response.status(),
            response.content()?,
            span,
        ));
    }

    let response_content = response.content()?;
    let content: Vec<JSONBucketSettings> = serde_json::from_str(&response_content)
        .map_err(|e| deserialize_error(e.to_string(), span))?;

    let mut buckets: Vec<BucketSettings> = vec![];
    for bucket in content.into_iter() {
        buckets.push(BucketSettings::try_from(bucket).map_err(|e| {
            malformed_response_error(
                "Could not parse bucket settings",
                format!("Error: {}, response content: {}", e, response_content),
                span,
            )
        })?);
    }

    Ok(buckets)
}
