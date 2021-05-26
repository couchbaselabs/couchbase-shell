//! The `ping` command performs a ping operation.

use crate::cli::util::cluster_identifiers_from;
use crate::state::State;

use crate::client::ServiceType;
use async_trait::async_trait;
use log::debug;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue};
use nu_source::Tag;
use nu_stream::OutputStream;
use std::ops::Add;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::time::Instant;

pub struct Ping {
    state: Arc<State>,
}

impl Ping {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for Ping {
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
    }

    fn usage(&self) -> &str {
        "Ping available services in the cluster"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        run_ping(self.state.clone(), args)
    }
}

fn run_ping(state: Arc<State>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();
    let args = args.evaluate_once()?;

    let bucket_name = match args
        .call_info
        .args
        .get("bucket")
        .map(|id| id.as_string().ok())
        .flatten()
        .or_else(|| state.active_cluster().active_bucket())
    {
        Some(v) => v,
        None => {
            return Err(ShellError::untagged_runtime_error(
                "Could not auto-select a bucket - please use --bucket instead".to_string(),
            ))
        }
    };

    let cluster_identifiers = cluster_identifiers_from(&state, &args, true)?;

    debug!("Running ping");

    let rt = Runtime::new().unwrap();
    let clusters_len = cluster_identifiers.len();
    let mut results = vec![];
    for identifier in cluster_identifiers {
        let cluster = match state.clusters().get(&identifier) {
            Some(c) => c,
            None => continue, //This can't actually happen, we filter the clusters in cluster_identifiers_from
        };
        let deadline = Instant::now().add(cluster.timeouts().query_timeout());
        let result = cluster
            .cluster()
            .ping_all_http_request(deadline, ctrl_c.clone());
        match result {
            Ok(res) => {
                for ping in res {
                    let tag = Tag::default();
                    let mut collected = TaggedDictBuilder::new(&tag);
                    if clusters_len > 1 {
                        collected.insert_value("cluster", identifier.clone());
                    }
                    collected.insert_value("service", ping.service().as_string());
                    collected.insert_value("remote", ping.address().to_string());
                    collected.insert_value(
                        "latency",
                        UntaggedValue::duration(ping.latency().as_nanos()).into_untagged_value(),
                    );
                    collected.insert_value("state", ping.state().to_string());

                    let error = match ping.error() {
                        Some(e) => e.to_string(),
                        None => "".into(),
                    };

                    collected.insert_value("error", error);
                    results.push(collected.into_value());
                }
            }
            Err(_e) => {}
        };

        // TODO: do this in parallel to http ops.
        let kv_deadline = Instant::now().add(cluster.timeouts().data_timeout());
        let mut client = match cluster.cluster().key_value_client(
            cluster.username().to_string(),
            cluster.password().to_string(),
            bucket_name.clone(),
            "".into(),
            "".into(),
            kv_deadline,
            ctrl_c.clone(),
        ) {
            Ok(c) => c,
            Err(e) => {
                let tag = Tag::default();
                let mut collected = TaggedDictBuilder::new(&tag);
                if clusters_len > 1 {
                    collected.insert_value("cluster", identifier.clone());
                }
                collected.insert_value("service", ServiceType::KeyValue.as_string());
                collected.insert_value("remote", "".to_string());
                collected.insert_value("latency", "".to_string());
                collected.insert_value("state", "error".to_string());

                collected.insert_value("error", e.to_string());
                results.push(collected.into_value());
                continue;
            }
        };

        let kv_result = rt.block_on(client.ping_all(kv_deadline, ctrl_c.clone()));
        match kv_result {
            Ok(res) => {
                for ping in res {
                    let tag = Tag::default();
                    let mut collected = TaggedDictBuilder::new(&tag);
                    if clusters_len > 1 {
                        collected.insert_value("cluster", identifier.clone());
                    }
                    collected.insert_value("service", ping.service().as_string());
                    collected.insert_value("remote", ping.address().to_string());
                    collected.insert_value(
                        "latency",
                        UntaggedValue::duration(ping.latency().as_nanos()).into_untagged_value(),
                    );
                    collected.insert_value("state", ping.state().to_string());

                    let error = match ping.error() {
                        Some(e) => e.to_string(),
                        None => "".into(),
                    };

                    collected.insert_value("error", error);
                    results.push(collected.into_value());
                }
            }
            Err(_e) => {}
        };
    }
    Ok(OutputStream::from(results))
}
