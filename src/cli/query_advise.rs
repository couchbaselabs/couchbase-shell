use crate::cli::util::convert_json_value_to_nu_value;
use crate::client::QueryRequest;
use crate::state::State;
use futures::executor::block_on;
use log::debug;
use nu_cli::ActionStream;
use nu_engine::CommandArgs;
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

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        run(self.state.clone(), args)
    }
}

fn run(state: Arc<State>, args: CommandArgs) -> Result<ActionStream, ShellError> {
    let args = args.evaluate_once()?;
    //let ctrl_c = args.ctrl_c.clone();

    let statement = args.nth(0).expect("need statement").as_string()?;
    let statement = format!("ADVISE {}", statement);

    let active_cluster = match args.call_info.args.get("cluster") {
        Some(c) => {
            let identifier = match c.as_string() {
                Ok(s) => s,
                Err(e) => {
                    return Err(ShellError::untagged_runtime_error(format!(
                        "Could not convert cluster name to string: {}",
                        e
                    )));
                }
            };
            match state.clusters().get(identifier.as_str()) {
                Some(c) => c,
                None => {
                    return Err(ShellError::untagged_runtime_error(format!(
                        "Could not get cluster from available clusters",
                    )));
                }
            }
        }
        None => state.active_cluster(),
    };

    debug!("Running n1ql query {}", &statement);
    let response = block_on(
        active_cluster
            .cluster()
            .query_request(QueryRequest::Execute {
                statement: statement.clone(),
                scope: None,
            }),
    )?;

    let content: serde_json::Value = serde_json::from_str(response.content())?;
    let converted = convert_json_value_to_nu_value(&content, Tag::default())?;
    Ok(ActionStream::one(converted))
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
