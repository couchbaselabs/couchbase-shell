use crate::cli::util::convert_json_value_to_nu_value;
use crate::client::AnalyticsQueryRequest;
use crate::state::State;
use async_trait::async_trait;
use log::debug;
use nu_cli::ActionStream;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::Signature;
use nu_source::Tag;
use std::ops::Add;
use std::sync::Arc;
use tokio::time::Instant;

pub struct AnalyticsIndexes {
    state: Arc<State>,
}

impl AnalyticsIndexes {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for AnalyticsIndexes {
    fn name(&self) -> &str {
        "analytics indexes"
    }

    fn signature(&self) -> Signature {
        Signature::build("analytics indexes")
    }

    fn usage(&self) -> &str {
        "Lists all analytics indexes"
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        indexes(self.state.clone(), args)
    }
}

fn indexes(state: Arc<State>, _args: CommandArgs) -> Result<ActionStream, ShellError> {
    let statement = "SELECT d.* FROM Metadata.`Index` d WHERE d.DataverseName <> \"Metadata\"";

    let active_cluster = state.active_cluster();
    debug!("Running analytics query {}", &statement);

    let response = active_cluster.cluster().analytics_query_request(
        AnalyticsQueryRequest::Execute {
            statement: statement.into(),
            scope: None,
        },
        Instant::now().add(active_cluster.timeouts().query_timeout()),
    )?;

    let content: serde_json::Value = serde_json::from_str(response.content())?;
    let converted = convert_json_value_to_nu_value(&content, Tag::default())?;
    Ok(ActionStream::one(converted))
}
