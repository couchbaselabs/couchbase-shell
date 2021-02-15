use super::util::convert_couchbase_rows_json_to_nu_stream;
use crate::state::State;
use async_trait::async_trait;
use couchbase::AnalyticsOptions;
use log::debug;
use nu_cli::OutputStream;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::Signature;
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
impl nu_engine::WholeStreamCommand for AnalyticsDataverses {
    fn name(&self) -> &str {
        "analytics dataverses"
    }

    fn signature(&self) -> Signature {
        Signature::build("analytics dataverses")
    }

    fn usage(&self) -> &str {
        "Lists all analytics dataverses"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        dataverses(self.state.clone(), args).await
    }
}

async fn dataverses(state: Arc<State>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once().await?;
    let ctrl_c = args.ctrl_c.clone();
    let statement = "SELECT d.* FROM Metadata.`Dataverse` d WHERE d.DataverseName <> \"Metadata\"";

    debug!("Running analytics query {}", &statement);
    let result = state
        .active_cluster()
        .cluster()
        .analytics_query(statement, AnalyticsOptions::default())
        .await;

    match result {
        Ok(mut r) => convert_couchbase_rows_json_to_nu_stream(ctrl_c, r.rows()),
        Err(e) => Err(ShellError::untagged_runtime_error(format!("{}", e))),
    }
}
