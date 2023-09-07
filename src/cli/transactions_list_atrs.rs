use crate::cli::error::{
    client_error_to_shell_error, deserialize_error, no_active_bucket_error,
    no_active_cluster_error, unexpected_status_code_error,
};
use crate::cli::util::{convert_json_value_to_nu_value, duration_to_golang_string};
use crate::client::QueryRequest;
use crate::state::State;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};
use std::collections::HashMap;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

// For now you need a covered index like
// create index id3 on `travel-sample`(meta().id, meta().xattrs.attempts);
#[derive(Clone)]
pub struct TransactionsListAtrs {
    state: Arc<Mutex<State>>,
}

impl TransactionsListAtrs {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for TransactionsListAtrs {
    fn name(&self) -> &str {
        "transactions list-atrs"
    }

    fn signature(&self) -> Signature {
        Signature::build("transactions list-atrs")
            .named(
                "bucket",
                SyntaxShape::String,
                "the name of the bucket",
                None,
            )
            .category(Category::Custom("couchbase".to_string()))
        /* .named("scope", SyntaxShape::String, "the name of the scope", None)
        .named(
            "collection",
            SyntaxShape::String,
            "the name of the collection",
            None,
        )*/
    }

    fn usage(&self) -> &str {
        "Lists all active transaction records"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;
        let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();

        let guard = self.state.lock().unwrap();
        let active_cluster = match guard.active_cluster() {
            Some(c) => c,
            None => {
                return Err(no_active_cluster_error(span));
            }
        };
        let bucket = match call
            .get_flag(engine_state, stack, "bucket")?
            .or_else(|| active_cluster.active_bucket())
        {
            Some(v) => Ok(v),
            None => Err(no_active_bucket_error(span)),
        }?;

        /*
        let scope = match args.get_flag("scope")? {
            Some(s) => s,
            None => match active_cluster.active_scope() {
                Some(s) => s,
                None => "".into(),
            },
        };

        let collection = match args.get_flag("collection")? {
            Some(c) => c,
            None => match active_cluster.active_collection() {
                Some(c) => c,
                None => "".into(),
            },
        };*/

        let statement = format!(
            "select meta().id, meta().xattrs.attempts from `{}` where meta().id like '_txn:atr%'",
            bucket
        );
        let response = active_cluster
            .cluster()
            .http_client()
            .query_request(
                QueryRequest::Execute {
                    statement,
                    scope: None,
                    timeout: duration_to_golang_string(active_cluster.timeouts().query_timeout()),
                    transaction: None,
                },
                Instant::now().add(active_cluster.timeouts().query_timeout()),
                ctrl_c,
            )
            .map_err(|e| client_error_to_shell_error(e, span))?;

        match response.status() {
            200 => {}
            _ => {
                return Err(unexpected_status_code_error(
                    response.status(),
                    response.content(),
                    span,
                ));
            }
        }

        let mut content: HashMap<String, serde_json::Value> =
            serde_json::from_str(response.content())
                .map_err(|e| deserialize_error(e.to_string(), span))?;
        let removed = if content.contains_key("errors") {
            content.remove("errors").unwrap()
        } else {
            content.remove("results").unwrap()
        };

        let values = removed
            .as_array()
            .unwrap()
            .iter()
            .map(|a| convert_json_value_to_nu_value(a, span).unwrap())
            .collect::<Vec<_>>();

        Ok(Value::List { vals: values, span }.into_pipeline_data())
    }
}
