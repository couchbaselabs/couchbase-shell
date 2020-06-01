use super::util::convert_json_value_to_nu_value;
use crate::state::State;
use couchbase::AnalyticsOptions;
use futures::stream::StreamExt;
use log::debug;
use nu_cli::{CommandArgs, CommandRegistry, OutputStream};
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_source::Tag;
use std::sync::Arc;
use async_trait::async_trait;

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

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        run(self.state.clone(), args, registry).await
    }
}

async fn run(
    state: Arc<State>,
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry).await?;
    let statement = args.nth(0).expect("need statement").as_string()?;

    debug!("Running analytics query {}", &statement);
    let mut result = state
        .active_cluster()
        .cluster()
        .analytics_query(statement, AnalyticsOptions::default())
        .await
        .unwrap();
    let stream = result
        .rows::<serde_json::Value>()
        .map(|v| convert_json_value_to_nu_value(&v.unwrap(), Tag::default()));
    Ok(OutputStream::from_input(stream))
}
