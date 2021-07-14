use crate::cli::util::{
    cluster_identifiers_from, convert_json_value_to_nu_value, convert_row_to_nu_value,
};
use crate::client::QueryRequest;
use crate::state::State;
use log::debug;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, Value};
use nu_source::Tag;
use nu_stream::OutputStream;
use serde::Deserialize;
use std::collections::HashMap;
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
            .named(
                "clusters",
                SyntaxShape::String,
                "the clusters to query against",
                None,
            )
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
    let with_meta = args.has_flag("with-meta");

    let statement: String = args.req(0)?;
    let statement = format!("ADVISE {}", statement);

    let cluster_identifiers = cluster_identifiers_from(&state, &args, true)?;
    let guard = state.lock().unwrap();
    debug!("Running n1ql query {}", &statement);

    let mut results: Vec<Value> = vec![];
    for identifier in cluster_identifiers {
        let active_cluster = match guard.clusters().get(&identifier) {
            Some(c) => c,
            None => {
                return Err(ShellError::unexpected("Cluster not found"));
            }
        };
        let response = active_cluster.cluster().http_client().query_request(
            QueryRequest::Execute {
                statement: statement.clone(),
                scope: None,
            },
            Instant::now().add(active_cluster.timeouts().query_timeout()),
            ctrl_c.clone(),
        )?;

        if with_meta {
            let content: serde_json::Value = serde_json::from_str(response.content())?;
            results.push(convert_row_to_nu_value(
                &content,
                Tag::default(),
                identifier.clone(),
            )?);
        } else {
            let content: HashMap<String, serde_json::Value> =
                serde_json::from_str(response.content())?;
            if let Some(content_errors) = content.get("errors") {
                if let Some(arr) = content_errors.as_array() {
                    for result in arr {
                        results.push(convert_row_to_nu_value(
                            result,
                            Tag::default(),
                            identifier.clone(),
                        )?);
                    }
                } else {
                    return Err(ShellError::unexpected(
                        "Query errors not an array - malformed response",
                    ));
                }
            } else if let Some(content_results) = content.get("results") {
                if let Some(arr) = content_results.as_array() {
                    for result in arr {
                        results
                            .push(convert_json_value_to_nu_value(result, Tag::default()).unwrap());
                    }
                } else {
                    return Err(ShellError::unexpected(
                        "Query results not an array - malformed response",
                    ));
                }
            } else {
                // Queries like "create index" can end up here.
                continue;
            };
        }
    }
    Ok(OutputStream::from(results))
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
