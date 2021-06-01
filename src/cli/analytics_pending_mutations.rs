use crate::cli::util::convert_json_value_to_nu_value;
use crate::client::AnalyticsQueryRequest;
use crate::state::State;
use async_trait::async_trait;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::Signature;
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
        Signature::build("analytics pending-mutations")
    }

    fn usage(&self) -> &str {
        "Lists all analytics pending mutations"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        dataverses(self.state.clone(), args)
    }
}

fn dataverses(state: Arc<Mutex<State>>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();

    let guard = state.lock().unwrap();
    let active_cluster = guard.active_cluster();

    let response = active_cluster.cluster().analytics_query_request(
        AnalyticsQueryRequest::PendingMutations,
        Instant::now().add(active_cluster.timeouts().query_timeout()),
        ctrl_c,
    )?;

    let content: serde_json::Value = serde_json::from_str(response.content())?;
    let converted = convert_json_value_to_nu_value(&content, Tag::default())?;
    Ok(OutputStream::one(converted))
}
