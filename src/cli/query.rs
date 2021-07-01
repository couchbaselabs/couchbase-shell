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
use std::collections::HashMap;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

pub struct Query {
    state: Arc<Mutex<State>>,
}

impl Query {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl nu_engine::WholeStreamCommand for Query {
    fn name(&self) -> &str {
        "query"
    }

    fn signature(&self) -> Signature {
        Signature::build("query")
            .required("statement", SyntaxShape::String, "the query statement")
            .named(
                "clusters",
                SyntaxShape::String,
                "the clusters to query against",
                None,
            )
            .named(
                "bucket",
                SyntaxShape::String,
                "the bucket to query against",
                None,
            )
            .named(
                "scope",
                SyntaxShape::String,
                "the scope to query against",
                None,
            )
            .switch("with-meta", "include toplevel metadata", None)
    }

    fn usage(&self) -> &str {
        "Performs a n1ql query"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        run(self.state.clone(), args)
    }
}

fn run(state: Arc<Mutex<State>>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();

    let cluster_identifiers = cluster_identifiers_from(&state, &args, true)?;
    let guard = state.lock().unwrap();
    let statement: String = args.req(0)?;

    let mut results: Vec<Value> = vec![];
    for identifier in cluster_identifiers {
        let active_cluster = match guard.clusters().get(&identifier) {
            Some(c) => c,
            None => {
                return Err(ShellError::untagged_runtime_error("Cluster not found"));
            }
        };
        let bucket = args
            .get_flag("bucket")?
            .or_else(|| active_cluster.active_bucket());

        let scope = args.get_flag("scope")?;

        let maybe_scope = bucket.map(|b| scope.map(|s| (b, s))).flatten();

        let with_meta = args.has_flag("with-meta");

        debug!("Running n1ql query {}", &statement);

        let response = active_cluster.cluster().http_client().query_request(
            QueryRequest::Execute {
                statement: statement.clone(),
                scope: maybe_scope,
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
                    return Err(ShellError::untagged_runtime_error(
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
                    return Err(ShellError::untagged_runtime_error(
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
