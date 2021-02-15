use super::util::convert_couchbase_rows_json_to_nu_stream;
use crate::state::State;
use async_trait::async_trait;
use couchbase::QueryOptions;
use log::debug;
use nu_cli::OutputStream;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use serde::Deserialize;
use std::sync::Arc;

pub struct QueryAdvise {
    state: Arc<State>,
}

impl QueryAdvise {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for QueryAdvise {
    fn name(&self) -> &str {
        "query advise"
    }

    fn signature(&self) -> Signature {
        Signature::build("query advise").required(
            "statement",
            SyntaxShape::String,
            "the query statement",
        )
    }

    fn usage(&self) -> &str {
        "Calls the query adviser and lists recommended indexes"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        run(self.state.clone(), args).await
    }
}

async fn run(state: Arc<State>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once().await?;
    let ctrl_c = args.ctrl_c.clone();

    let statement = args.nth(0).expect("need statement").as_string()?;

    let statement = format!("ADVISE {}", statement);

    debug!("Running n1ql query {}", &statement);
    let result = state
        .active_cluster()
        .cluster()
        .query(statement, QueryOptions::default())
        .await;

    match result {
        Ok(mut r) => convert_couchbase_rows_json_to_nu_stream(ctrl_c, r.rows()),
        Err(e) => Err(ShellError::untagged_runtime_error(format!("{}", e))),
    }
}

#[derive(Debug, Deserialize)]
struct AdviseResult {
    query: String,
    advice: Advice,
}

#[derive(Debug, Deserialize)]
struct Advice {
    adviseinfo: Vec<serde_json::Value>,
}
