use super::util::convert_json_value_to_nu_value;
use crate::state::State;
use couchbase::AnalyticsOptions;
use futures::executor::block_on;
use futures::stream::StreamExt;
use log::debug;
use nu_cli::{CommandArgs, CommandRegistry, OutputStream};
use nu_errors::ShellError;
use nu_protocol::Signature;
use nu_source::Tag;
use std::sync::Arc;

pub struct AnalyticsDatasets {
    state: Arc<State>,
}

impl AnalyticsDatasets {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

impl nu_cli::WholeStreamCommand for AnalyticsDatasets {
    fn name(&self) -> &str {
        "analytics datasets"
    }

    fn signature(&self) -> Signature {
        Signature::build("analytics datasets")
    }

    fn usage(&self) -> &str {
        "Lists all analytics datasets"
    }

    fn run(
        &self,
        _args: CommandArgs,
        _registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        block_on(datasets(self.state.clone()))
    }
}

async fn datasets(state: Arc<State>) -> Result<OutputStream, ShellError> {
    let statement = "SELECT d.* FROM Metadata.`Dataset` d WHERE d.DataverseName <> \"Metadata\"";

    debug!("Running analytics query {}", &statement);
    let result = state
        .active_cluster()
        .cluster()
        .analytics_query(statement, AnalyticsOptions::default())
        .await;

    match result {
        Ok(mut r) => {
            let stream = r
                .rows::<serde_json::Value>()
                .map(|v| convert_json_value_to_nu_value(&v.unwrap(), Tag::default()));
            Ok(OutputStream::from_input(stream))
        }
        Err(e) => Err(ShellError::untagged_runtime_error(format!("{}", e))),
    }
}
