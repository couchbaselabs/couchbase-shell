use crate::cli::util::{cluster_identifiers_from, convert_row_to_nu_value, validate_is_not_cloud};
use crate::client::AnalyticsQueryRequest;
use crate::state::State;
use async_trait::async_trait;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, Value};
use nu_source::Tag;
use nu_stream::OutputStream;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

pub struct AnalyticsPendingMutations {
    state: Arc<Mutex<State>>,
}

impl AnalyticsPendingMutations {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for AnalyticsPendingMutations {
    fn name(&self) -> &str {
        "analytics pending-mutations"
    }

    fn signature(&self) -> Signature {
        Signature::build("analytics pending-mutations").named(
            "clusters",
            SyntaxShape::String,
            "the clusters which should be contacted",
            None,
        )
    }

    fn usage(&self) -> &str {
        "Lists all analytics pending mutations"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        pending_mutations(self.state.clone(), args)
    }
}

fn pending_mutations(
    state: Arc<Mutex<State>>,
    args: CommandArgs,
) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();

    let cluster_identifiers = cluster_identifiers_from(&state, &args, true)?;
    let guard = state.lock().unwrap();

    let mut results: Vec<Value> = vec![];
    for identifier in cluster_identifiers {
        let active_cluster = match guard.clusters().get(&identifier) {
            Some(c) => c,
            None => {
                return Err(ShellError::unexpected("Cluster not found"));
            }
        };
        validate_is_not_cloud(
            active_cluster,
            "pending mutations cannot be run against Capella clusters",
        )?;

        let response = active_cluster
            .cluster()
            .http_client()
            .analytics_query_request(
                AnalyticsQueryRequest::PendingMutations,
                Instant::now().add(active_cluster.timeouts().analytics_timeout()),
                ctrl_c.clone(),
            )?;

        let content: serde_json::Value = serde_json::from_str(response.content())?;
        let converted = convert_row_to_nu_value(&content, Tag::default(), identifier.clone())?;
        results.push(converted);
    }
    Ok(OutputStream::from(results))
}
