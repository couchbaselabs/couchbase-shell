use super::util::convert_json_value_to_nu_value;
use crate::state::State;
use couchbase::QueryOptions;
use futures::executor::block_on;
use futures::stream::StreamExt;
use log::debug;
use nu_cli::{CommandArgs, CommandRegistry, OutputStream};
use nu_errors::ShellError;
use nu_protocol::Signature;
use nu_source::Tag;
use std::sync::Arc;

pub struct Indexes {
    state: Arc<State>,
}

impl Indexes {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

impl nu_cli::WholeStreamCommand for Indexes {
    fn name(&self) -> &str {
        "indexes"
    }

    fn signature(&self) -> Signature {
        Signature::build("indexes")
    }

    fn usage(&self) -> &str {
        "Lists all indexes"
    }

    fn run(
        &self,
        _args: CommandArgs,
        _registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        block_on(indexes(self.state.clone()))
    }
}

async fn indexes(state: Arc<State>) -> Result<OutputStream, ShellError> {
    let statement = "select keyspace_id as `bucket`, name, state, `using` as `type`, ifmissing(condition, null) as condition from system:indexes";

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
            Ok(OutputStream::from_input(stream))
        }
        Err(e) => Err(ShellError::untagged_runtime_error(format!("{}", e))),
    }
}
