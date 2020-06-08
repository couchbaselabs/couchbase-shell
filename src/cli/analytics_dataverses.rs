use super::util::convert_json_value_to_nu_value;
use crate::state::State;
use async_trait::async_trait;
use couchbase::AnalyticsOptions;
use futures::stream::StreamExt;
use log::debug;
use nu_cli::{CommandArgs, CommandRegistry, InterruptibleStream, OutputStream};
use nu_errors::ShellError;
use nu_protocol::Signature;
use nu_source::Tag;
use std::sync::Arc;

pub struct AnalyticsDataverses {
    state: Arc<State>,
}

impl AnalyticsDataverses {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_cli::WholeStreamCommand for AnalyticsDataverses {
    fn name(&self) -> &str {
        "analytics dataverses"
    }

    fn signature(&self) -> Signature {
        Signature::build("analytics dataverses")
    }

    fn usage(&self) -> &str {
        "Lists all analytics dataverses"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        dataverses(self.state.clone(), args, registry).await
    }
}

async fn dataverses(
    state: Arc<State>,
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry).await?;
    let ctrl_c = args.ctrl_c.clone();
    let statement = "SELECT d.* FROM Metadata.`Dataverse` d WHERE d.DataverseName <> \"Metadata\"";

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
            Ok(OutputStream::from_input(InterruptibleStream::new(
                stream, ctrl_c,
            )))
        }
        Err(e) => Err(ShellError::untagged_runtime_error(format!("{}", e))),
    }
}
