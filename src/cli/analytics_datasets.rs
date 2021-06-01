use crate::cli::util::convert_json_value_to_nu_value;
use crate::client::AnalyticsQueryRequest;
use crate::state::State;
use log::debug;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::Signature;
use nu_source::Tag;
use nu_stream::OutputStream;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

pub struct AnalyticsDatasets {
    state: Arc<Mutex<State>>,
}

impl AnalyticsDatasets {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl nu_engine::WholeStreamCommand for AnalyticsDatasets {
    fn name(&self) -> &str {
        "analytics datasets"
    }

    fn signature(&self) -> Signature {
        Signature::build("analytics datasets").switch(
            "with-meta",
            "Includes related metadata in the result",
            None,
        )
    }

    fn usage(&self) -> &str {
        "Lists all analytics datasets"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        datasets(self.state.clone(), args)
    }
}

fn datasets(state: Arc<Mutex<State>>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();
    let statement = "SELECT d.* FROM Metadata.`Dataset` d WHERE d.DataverseName <> \"Metadata\"";

    let guard = state.lock().unwrap();
    let active_cluster = guard.active_cluster();
    debug!("Running analytics query {}", &statement);

    let response = active_cluster.cluster().analytics_query_request(
        AnalyticsQueryRequest::Execute {
            statement: statement.into(),
            scope: None,
        },
        Instant::now().add(active_cluster.timeouts().query_timeout()),
        ctrl_c,
    )?;

    let with_meta = args.call_info().switch_present("with-meta");
    let content: serde_json::Value = serde_json::from_str(response.content())?;
    if with_meta {
        let converted = convert_json_value_to_nu_value(&content, Tag::default())?;
        Ok(OutputStream::one(converted))
    } else {
        if let Some(results) = content.get("results") {
            if let Some(arr) = results.as_array() {
                let mut converted = vec![];
                for result in arr {
                    converted.push(convert_json_value_to_nu_value(result, Tag::default())?);
                }
                Ok(OutputStream::from(converted))
            } else {
                Err(ShellError::untagged_runtime_error(
                    "Analytics result not an array - malformed response",
                ))
            }
        } else {
            Err(ShellError::untagged_runtime_error(
                "Analytics toplevel result not  an object - malformed response",
            ))
        }
    }
}
