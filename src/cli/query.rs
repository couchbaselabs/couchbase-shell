use crate::cli::util::convert_json_value_to_nu_value;
use crate::client::QueryRequest;
use crate::state::State;
use log::debug;
use nu_cli::ActionStream;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_source::Tag;
use std::sync::Arc;

pub struct Query {
    state: Arc<State>,
}

impl Query {
    pub fn new(state: Arc<State>) -> Self {
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
                "cluster",
                SyntaxShape::String,
                "the cluster to query against",
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
    }

    fn usage(&self) -> &str {
        "Performs a n1ql query"
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        run(self.state.clone(), args)
    }
}

fn run(state: Arc<State>, args: CommandArgs) -> Result<ActionStream, ShellError> {
    let args = args.evaluate_once()?;
    // let ctrl_c = args.ctrl_c.clone();
    let statement = args.nth(0).expect("need statement").as_string()?;
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
    let bucket = match args
        .call_info
        .args
        .get("bucket")
        .map(|bucket| bucket.as_string().ok())
        .flatten()
        .or_else(|| active_cluster.active_bucket())
    {
        Some(v) => Some(v),
        None => None,
    };
    let scope = match args.call_info.args.get("scope") {
        Some(v) => match v.as_string() {
            Ok(name) => Some(name),
            Err(e) => return Err(e),
        },
        None => None,
    };

    let maybe_scope = if bucket.is_some() && scope.is_some() {
        Some((bucket.unwrap().clone(), scope.unwrap().clone()))
    } else {
        None
    };

    debug!("Running n1ql query {}", &statement);

    let response = active_cluster
        .cluster()
        .query_request(QueryRequest::Execute {
            statement: statement.clone(),
            scope: maybe_scope,
        })?;

    let content: serde_json::Value = serde_json::from_str(response.content())?;
    let converted = convert_json_value_to_nu_value(&content, Tag::default())?;
    Ok(ActionStream::one(converted))
}
