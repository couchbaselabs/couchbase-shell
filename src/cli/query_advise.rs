use crate::cli::util::convert_json_value_to_nu_value;
use crate::client::QueryRequest;
use crate::state::State;
use log::debug;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_source::Tag;
use nu_stream::OutputStream;
use serde::Deserialize;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

pub struct QueryAdvise {
    state: Arc<Mutex<State>>,
}

impl QueryAdvise {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl nu_engine::WholeStreamCommand for QueryAdvise {
    fn name(&self) -> &str {
        "query advise"
    }

    fn signature(&self) -> Signature {
        Signature::build("query advise")
            .required("statement", SyntaxShape::String, "the query statement")
            .switch("with-meta", "Includes related metadata in the result", None)
    }

    fn usage(&self) -> &str {
        "Calls the query adviser and lists recommended indexes"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        run(self.state.clone(), args)
    }
}

fn run(state: Arc<Mutex<State>>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();
    let with_meta = args.call_info().switch_present("with-meta");
    let args = args.evaluate_once()?;

    let statement = args.nth(0).expect("need statement").as_string()?;
    let statement = format!("ADVISE {}", statement);

    let guard = state.lock().unwrap();
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
            match guard.clusters().get(identifier.as_str()) {
                Some(c) => c,
                None => {
                    return Err(ShellError::untagged_runtime_error(
                        "Could not get cluster from available clusters".to_string(),
                    ));
                }
            }
        }
        None => guard.active_cluster(),
    };

    debug!("Running n1ql query {}", &statement);

    let response = active_cluster.cluster().http_client().query_request(
        QueryRequest::Execute {
            statement,
            scope: None,
        },
        Instant::now().add(active_cluster.timeouts().query_timeout()),
        ctrl_c,
    )?;

    let content: serde_json::Value = serde_json::from_str(response.content())?;
    if with_meta {
        let converted = convert_json_value_to_nu_value(&content, Tag::default())?;
        Ok(OutputStream::one(converted))
    } else {
        if let Some(results) = content.get("results") {
            if let Some(arr) = results.as_array() {
                let mut converted = vec![];
                for result in arr {
                    converted.push(convert_json_value_to_nu_value(result, Tag::default())?);
                }
                Ok(OutputStream::from(converted))
            } else {
                Err(ShellError::untagged_runtime_error(
                    "Query result not an array - malformed response",
                ))
            }
        } else {
            Err(ShellError::untagged_runtime_error(
                "Query toplevel result not  an object - malformed response",
            ))
        }
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
