use crate::cli::util::convert_json_value_to_nu_value;
use crate::client::AnalyticsQueryRequest;
use crate::state::State;
use async_trait::async_trait;
use log::debug;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::Signature;
use nu_source::Tag;
use nu_stream::OutputStream;
use std::ops::Add;
use std::sync::Arc;
use tokio::time::Instant;

pub struct AnalyticsBuckets {
    state: Arc<State>,
}

impl AnalyticsBuckets {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for AnalyticsBuckets {
    fn name(&self) -> &str {
        "analytics buckets"
    }

    fn signature(&self) -> Signature {
        Signature::build("analytics buckets")
    }

    fn usage(&self) -> &str {
        "Lists all analytics buckets"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        dataverses(self.state.clone(), args)
    }
}

fn dataverses(state: Arc<State>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();
    let statement = "SELECT `Bucket`.* FROM `Metadata`.`Bucket`";

    let active_cluster = state.active_cluster();
    debug!("Running analytics query {}", &statement);

    let response = active_cluster.cluster().analytics_query_request(
        AnalyticsQueryRequest::Execute {
            statement: statement.into(),
            scope: None,
        },
        Instant::now().add(active_cluster.timeouts().query_timeout()),
        ctrl_c,
    )?;

    let content: serde_json::Value = serde_json::from_str(response.content())?;
    let converted = convert_json_value_to_nu_value(&content, Tag::default())?;
    Ok(OutputStream::one(converted))
}
