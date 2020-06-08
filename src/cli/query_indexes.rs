use super::util::convert_json_value_to_nu_value;
use crate::state::State;
use async_trait::async_trait;
use couchbase::QueryOptions;
use futures::stream::StreamExt;
use log::debug;
use nu_cli::{CommandArgs, CommandRegistry, InterruptibleStream, OutputStream};
use nu_errors::ShellError;
use nu_protocol::Signature;
use nu_source::Tag;
use std::sync::Arc;

pub struct QueryIndexes {
    state: Arc<State>,
}

impl QueryIndexes {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_cli::WholeStreamCommand for QueryIndexes {
    fn name(&self) -> &str {
        "query indexes"
    }

    fn signature(&self) -> Signature {
        Signature::build("query indexes")
    }

    fn usage(&self) -> &str {
        "Lists all query indexes"
    }

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        indexes(self.state.clone(), args, registry).await
    }
}

async fn indexes(
    state: Arc<State>,
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry).await?;
    let ctrl_c = args.ctrl_c.clone();

    let statement = "select keyspace_id as `bucket`, name, state, `using` as `type`, ifmissing(condition, null) as condition, ifmissing(is_primary, false) as `primary`, index_key from system:indexes";

    debug!("Running n1ql query {}", &statement);
    let result = state
        .active_cluster()
        .cluster()
        .query(statement, QueryOptions::default())
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
