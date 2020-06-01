use super::util::convert_json_value_to_nu_value;
use crate::state::State;
use async_trait::async_trait;
use couchbase::QueryOptions;
use futures::stream::StreamExt;
use log::debug;
use nu_cli::{CommandArgs, CommandRegistry, OutputStream};
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_source::Tag;
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
impl nu_cli::WholeStreamCommand for QueryAdvise {
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

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        run(self.state.clone(), args, registry).await
    }
}

async fn run(
    state: Arc<State>,
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry).await?;
    let statement = args.nth(0).expect("need statement").as_string()?;

    let statement = format!("ADVISE {}", statement);

    debug!("Running n1ql query {}", &statement);
    let result = state
        .active_cluster()
        .cluster()
        .query(statement, QueryOptions::default())
        .await;

    match result {
        Ok(mut r) => {
            let stream = r
                .rows::<AdviseResult>()
                .flat_map(|r| {
                    let advices = r.unwrap().advice.adviseinfo;
                    futures::stream::iter(advices)
                })
                .map(|v| convert_json_value_to_nu_value(&v, Tag::default()));
            Ok(OutputStream::from_input(stream))
        }
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
