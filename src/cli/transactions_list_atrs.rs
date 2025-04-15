use crate::cli::deserialize_error;
use crate::cli::error::{
    client_error_to_shell_error, no_active_bucket_error, no_active_cluster_error,
};
use crate::cli::query::send_query;
use crate::cli::util::convert_json_value_to_nu_value;
use crate::state::State;
use nu_engine::command_prelude::Call;
use nu_engine::CallExt;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;

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

    fn description(&self) -> &str {
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
        let signals = engine_state.signals().clone();

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
        let rt = Runtime::new()?;

        let contents = rt.block_on(async {
            let mut response = send_query(
                active_cluster,
                statement.clone(),
                None,
                None,
                signals.clone(),
                None,
                span,
                None,
            )
            .await?;

            response
                .content()
                .await
                .map_err(|e| client_error_to_shell_error(e, span))
        })?;

        let mut values = vec![];

        for content in contents {
            let content = serde_json::from_slice(&content)
                .map_err(|e| deserialize_error(e.to_string(), span))?;
            let content = convert_json_value_to_nu_value(&content, span)?;

            values.push(content);
        }

        Ok(Value::List {
            vals: values,
            internal_span: span,
        }
        .into_pipeline_data())
    }
}
