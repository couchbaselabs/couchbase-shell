//! The `ping` command performs a ping operation.

use crate::cli::util::{cluster_identifiers_from, get_active_cluster, NuValueMap};
use crate::state::State;

use log::debug;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;
use tokio::time::Instant;

use crate::cli::error::no_active_bucket_error;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct Ping {
    state: Arc<Mutex<State>>,
}

impl Ping {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for Ping {
    fn name(&self) -> &str {
        "ping"
    }

    fn signature(&self) -> Signature {
        Signature::build("ping")
            .named(
                "bucket",
                SyntaxShape::String,
                "the name of the bucket",
                None,
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
        "Ping available services in the cluster"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        run_ping(self.state.clone(), engine_state, stack, call, input)
    }
}

fn run_ping(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();

    let cluster_identifiers = cluster_identifiers_from(&engine_state, stack, &state, &call, true)?;

    let guard = state.lock().unwrap();

    debug!("Running ping");

    let rt = Runtime::new().unwrap();
    let clusters_len = cluster_identifiers.len();
    let mut results = vec![];
    for identifier in cluster_identifiers {
        let cluster = get_active_cluster(identifier.clone(), &guard, span.clone())?;
        let deadline = Instant::now().add(cluster.timeouts().management_timeout());

        let client = cluster.cluster().http_client();
        let result = client.ping_all_request(deadline, ctrl_c.clone());
        match result {
            Ok(res) => {
                for ping in res {
                    let mut collected = NuValueMap::default();
                    if clusters_len > 1 {
                        collected.add_string("cluster", identifier.clone(), span);
                    }
                    collected.add_string("service", ping.service().as_string(), span);
                    collected.add_string("remote", ping.address().to_string(), span);
                    collected.add(
                        "latency",
                        Value::Duration {
                            val: ping.latency().as_nanos() as i64,
                            span,
                        },
                    );
                    collected.add_string("state", ping.state().to_string(), span);

                    let error = match ping.error() {
                        Some(e) => e.to_string(),
                        None => "".into(),
                    };

                    collected.add_string("error", error, span);
                    results.push(collected.into_value(span));
                }
            }
            Err(_e) => {}
        };
        let bucket_name = match call
            .get_flag(engine_state, stack, "bucket")?
            .or_else(|| cluster.active_bucket())
        {
            Some(v) => v,
            None => return Err(no_active_bucket_error(span)),
        };

        // TODO: do this in parallel to http ops.
        let kv_deadline = Instant::now().add(cluster.timeouts().data_timeout());
        let mut client = rt.block_on(cluster.cluster().key_value_client(
            bucket_name.clone(),
            kv_deadline,
            ctrl_c.clone(),
            span,
        ))?;

        let kv_result = rt.block_on(client.ping_all(kv_deadline, ctrl_c.clone()));
        match kv_result {
            Ok(res) => {
                for ping in res {
                    let mut collected = NuValueMap::default();
                    if clusters_len > 1 {
                        collected.add_string("cluster", identifier.clone(), span);
                    }
                    collected.add_string("service", ping.service().as_string(), span);
                    collected.add_string("remote", ping.address().to_string(), span);
                    collected.add(
                        "latency",
                        Value::Duration {
                            val: ping.latency().as_nanos() as i64,
                            span,
                        },
                    );
                    collected.add_string("state", ping.state().to_string(), span);

                    let error = match ping.error() {
                        Some(e) => e.to_string(),
                        None => "".into(),
                    };

                    collected.add_string("error", error, span);
                    results.push(collected.into_value(span));
                }
            }
            Err(_e) => {}
        };
    }

    Ok(Value::List {
        vals: results,
        span: call.head,
    }
    .into_pipeline_data())
}
