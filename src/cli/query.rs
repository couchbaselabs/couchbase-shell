use super::util::convert_couchbase_rows_json_to_nu_stream;
use crate::state::State;
use async_trait::async_trait;
use couchbase::QueryOptions;
use log::debug;
use nu_cli::OutputStream;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use std::sync::Arc;

pub struct Query {
    state: Arc<State>,
}

impl Query {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for Query {
    fn name(&self) -> &str {
        "query"
    }

    fn signature(&self) -> Signature {
        Signature::build("query")
            .required("statement", SyntaxShape::String, "the query statement")
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

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        run(self.state.clone(), args).await
    }
}

async fn run(state: Arc<State>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once().await?;
    let ctrl_c = args.ctrl_c.clone();
    let statement = args.nth(0).expect("need statement").as_string()?;
    let active_cluster = state.active_cluster();
    let bucket = match args
        .get("bucket")
        .map(|bucket| bucket.as_string().ok())
        .flatten()
        .or_else(|| active_cluster.active_bucket())
    {
        Some(v) => Some(v),
        None => None,
    };
    let scope = match args.get("scope") {
        Some(v) => match v.as_string() {
            Ok(name) => Some(name),
            Err(e) => return Err(e),
        },
        None => None,
    };

    let scope_instance = match scope {
        Some(s) => match bucket {
            Some(b) => Some(active_cluster.bucket(b.as_str()).scope(s)),
            None => match active_cluster.active_bucket() {
                Some(b) => Some(active_cluster.bucket(b.as_str()).scope(s)),
                None => {
                    return Err(ShellError::untagged_runtime_error(format!(
                        "Could not auto-select a bucket - please use --bucket instead"
                    )));
                }
            },
        },
        None => None,
    };

    debug!("Running n1ql query {}", &statement);
    let result = match scope_instance {
        Some(s) => s.query(statement, QueryOptions::default()).await,
        None => {
            active_cluster
                .cluster()
                .query(statement, QueryOptions::default())
                .await
        }
    };

    match result {
        Ok(mut r) => convert_couchbase_rows_json_to_nu_stream(ctrl_c, r.rows()),
        Err(e) => Err(ShellError::untagged_runtime_error(format!("{}", e))),
    }
}
