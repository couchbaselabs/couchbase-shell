use super::util::convert_couchbase_rows_json_to_nu_stream;
use crate::state::State;
use async_trait::async_trait;
use couchbase::AnalyticsOptions;
use log::debug;
use nu_cli::{CommandArgs, OutputStream};
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use std::sync::Arc;

pub struct Analytics {
    state: Arc<State>,
}

impl Analytics {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_cli::WholeStreamCommand for Analytics {
    fn name(&self) -> &str {
        "analytics"
    }

    fn signature(&self) -> Signature {
        Signature::build("analytics").required(
            "statement",
            SyntaxShape::String,
            "the analytics statement",
        )
    }

    fn usage(&self) -> &str {
        "Performs an analytics query"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        run(self.state.clone(), args).await
    }
}

async fn run(state: Arc<State>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once().await?;
    let ctrl_c = args.ctrl_c.clone();
    let statement = args.nth(0).expect("need statement").as_string()?;

    debug!("Running analytics query {}", &statement);
    let mut result = match state
        .active_cluster()
        .cluster()
        .analytics_query(statement, AnalyticsOptions::default())
        .await
    {
        Ok(r) => r,
        Err(e) => {
            return Err(ShellError::untagged_runtime_error(format!("{}", e)));
        }
    };

    convert_couchbase_rows_json_to_nu_stream(ctrl_c, result.rows())
}
