use crate::cli::util::{cluster_identifiers_from, convert_row_to_nu_value};
use crate::client::AnalyticsQueryRequest;
use crate::state::State;
use async_trait::async_trait;
use log::debug;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, Value};
use nu_source::Tag;
use nu_stream::OutputStream;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

pub struct AnalyticsIndexes {
    state: Arc<Mutex<State>>,
}

impl AnalyticsIndexes {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for AnalyticsIndexes {
    fn name(&self) -> &str {
        "analytics indexes"
    }

    fn signature(&self) -> Signature {
        Signature::build("analytics indexes")
            .switch("with-meta", "Includes related metadata in the result", None)
            .named(
                "clusters",
                SyntaxShape::String,
                "the clusters which should be contacted",
                None,
            )
    }

    fn usage(&self) -> &str {
        "Lists all analytics indexes"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        indexes(self.state.clone(), args)
    }
}

fn indexes(state: Arc<Mutex<State>>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();
    let statement = "SELECT d.* FROM Metadata.`Index` d WHERE d.DataverseName <> \"Metadata\"";

    let cluster_identifiers = cluster_identifiers_from(&state, &args, true)?;
    let guard = state.lock().unwrap();
    debug!("Running analytics query {}", &statement);

    let mut results: Vec<Value> = vec![];
    for identifier in cluster_identifiers {
        let active_cluster = match guard.clusters().get(&identifier) {
            Some(c) => c,
            None => {
                return Err(ShellError::unexpected("Cluster not found"));
            }
        };

        let response = active_cluster
            .cluster()
            .http_client()
            .analytics_query_request(
                AnalyticsQueryRequest::Execute {
                    statement: statement.into(),
                    scope: None,
                },
                Instant::now().add(active_cluster.timeouts().analytics_timeout()),
                ctrl_c.clone(),
            )?;

        let with_meta = args.call_info().switch_present("with-meta");
        let content: serde_json::Value = serde_json::from_str(response.content())?;
        if with_meta {
            let converted = convert_row_to_nu_value(&content, Tag::default(), identifier.clone())?;
            results.push(converted);
        } else if let Some(results) = content.get("results") {
            if let Some(arr) = results.as_array() {
                let mut converted = vec![];
                for result in arr {
                    converted.push(convert_row_to_nu_value(
                        result,
                        Tag::default(),
                        identifier.clone(),
                    )?);
                }
            } else {
                return Err(ShellError::unexpected(
                    "Analytics result not an array - malformed response",
                ));
            }
        } else {
            return Err(ShellError::unexpected(
                "Analytics toplevel result not  an object - malformed response",
            ));
        }
    }
    Ok(OutputStream::from(results))
}
