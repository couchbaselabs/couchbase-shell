use crate::cli::util::convert_json_value_to_nu_value;
use crate::client::QueryRequest;
use crate::state::State;
use log::debug;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
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

    let guard = state.lock().unwrap();
    let statement: String = args.req(0)?;
    let cluster: Option<String> = args.get_flag("cluster")?;
    let active_cluster = match cluster {
        Some(identifier) => match guard.clusters().get(identifier.as_str()) {
            Some(c) => c,
            None => {
                return Err(ShellError::untagged_runtime_error(
                    "Could not get cluster from available clusters".to_string(),
                ));
            }
        },
        None => guard.active_cluster(),
    };
    let bucket = args
        .get_flag("bucket")?
        .or_else(|| active_cluster.active_bucket());

    let scope = args.get_flag("scope")?;

    let maybe_scope = bucket.map(|b| scope.map(|s| (b, s))).flatten();

    let with_meta = args.get_flag::<bool>("with-meta").unwrap().is_some();

    debug!("Running n1ql query {}", &statement);

    let response = active_cluster.cluster().http_client().query_request(
        QueryRequest::Execute {
            statement,
            scope: maybe_scope,
        },
        Instant::now().add(active_cluster.timeouts().query_timeout()),
        ctrl_c,
    )?;

    if with_meta {
        let content: serde_json::Value = serde_json::from_str(response.content())?;
        Ok(OutputStream::one(convert_json_value_to_nu_value(
            &content,
            Tag::default(),
        )?))
    } else {
        let mut content: HashMap<String, serde_json::Value> =
            serde_json::from_str(response.content())?;
        let removed = if content.contains_key("errors") {
            content.remove("errors").unwrap()
        } else {
            content.remove("results").unwrap()
        };

        let values = removed
            .as_array()
            .unwrap()
            .iter()
            .map(|a| convert_json_value_to_nu_value(a, Tag::default()).unwrap())
            .collect::<Vec<_>>();
        Ok(OutputStream::from(values))
    }
}
