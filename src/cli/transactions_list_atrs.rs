use crate::cli::util::convert_json_value_to_nu_value;
use crate::client::QueryRequest;
use crate::state::State;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_source::Tag;
use nu_stream::OutputStream;
use std::collections::HashMap;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

// For now you need a covered index like
// create index id3 on `travel-sample`(meta().id, meta().xattrs.attempts);
pub struct TransactionsListAtrs {
    state: Arc<Mutex<State>>,
}

impl TransactionsListAtrs {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl nu_engine::WholeStreamCommand for TransactionsListAtrs {
    fn name(&self) -> &str {
        "transactions list-atrs"
    }

    fn signature(&self) -> Signature {
        Signature::build("transactions list-atrs").named(
            "bucket",
            SyntaxShape::String,
            "the name of the bucket",
            None,
        )
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

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let ctrl_c = args.ctrl_c();

        let guard = self.state.lock().unwrap();
        let active_cluster = guard.active_cluster();
        let bucket = match args
            .get_flag("bucket")?
            .or_else(|| active_cluster.active_bucket())
        {
            Some(v) => Ok(v),
            None => Err(ShellError::untagged_runtime_error(
                "Could not auto-select a bucket - please use --bucket instead".to_string(),
            )),
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
        let response = active_cluster.cluster().http_client().query_request(
            QueryRequest::Execute {
                statement,
                scope: None,
            },
            Instant::now().add(active_cluster.timeouts().management_timeout()),
            ctrl_c,
        )?;
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
