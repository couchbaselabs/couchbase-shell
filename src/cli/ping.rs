//! The `ping` command performs a ping operation.

use crate::cli::util::cluster_identifiers_from;
use crate::state::State;
use couchbase::PingOptions;

use async_trait::async_trait;
use log::debug;
use nu_cli::{CommandArgs, CommandRegistry, OutputStream};
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue};
use nu_source::Tag;
use std::sync::Arc;

pub struct Ping {
    state: Arc<State>,
}

impl Ping {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_cli::WholeStreamCommand for Ping {
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

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        run_ping(self.state.clone(), args, registry).await
    }
}

async fn run_ping(
    state: Arc<State>,
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry).await?;

    let bucket_name = match args
        .get("bucket")
        .map(|id| id.as_string().unwrap())
        .or_else(|| state.active_cluster().active_bucket())
    {
        Some(v) => v,
        None => {
            return Err(ShellError::untagged_runtime_error(format!(
                "Could not auto-select a bucket - please use --bucket instead"
            )))
        }
    };

    let identifier_arg = args
        .get("clusters")
        .map(|id| id.as_string().unwrap())
        .unwrap_or_else(|| state.active());

    let cluster_identifiers = cluster_identifiers_from(&state, identifier_arg.as_str());

    debug!("Running ping");

    let clusters_len = cluster_identifiers.len();
    let mut results = vec![];
    for identifier in cluster_identifiers {
        let bucket = state
            .clusters()
            .get(&identifier)
            .unwrap()
            .cluster()
            .bucket(&bucket_name);
        match bucket.ping(PingOptions::default()).await {
            Ok(res) => {
                for (service_type, endpoints) in res.endpoints().iter() {
                    for endpoint in endpoints {
                        let tag = Tag::default();
                        let mut collected = TaggedDictBuilder::new(&tag);
                        if clusters_len > 1 {
                            collected.insert_value("cluster", identifier.clone());
                        }
                        collected.insert_value("service", service_type.to_string());
                        collected.insert_value("conn id", endpoint.id());
                        collected.insert_value("local", endpoint.local().unwrap_or_default());
                        collected.insert_value("remote", endpoint.remote().unwrap_or_default());
                        collected.insert_value(
                            "latency",
                            UntaggedValue::duration(endpoint.latency().as_secs() as i64)
                                .into_untagged_value(),
                        );
                        collected.insert_value("state", endpoint.state().to_string());
                        collected.insert_value("error", endpoint.error().unwrap_or_default());
                        collected.insert_value("bucket", endpoint.namespace().unwrap_or_default());
                        results.push(collected.into_value());
                    }
                }
            }
            Err(_e) => {}
        };
    }
    Ok(OutputStream::from(results))
}
