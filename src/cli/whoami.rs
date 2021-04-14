use super::util::convert_json_value_to_nu_value;
use crate::cli::util::cluster_identifiers_from;
use crate::client::ManagementRequest;
use crate::state::State;
use async_trait::async_trait;
use futures::channel::oneshot;
use futures::executor::block_on;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_source::Tag;
use nu_stream::OutputStream;
use serde_json::{json, Map, Value};
use std::sync::Arc;

pub struct Whoami {
    state: Arc<State>,
}

impl Whoami {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for Whoami {
    fn name(&self) -> &str {
        "whoami"
    }

    fn signature(&self) -> Signature {
        Signature::build("whoami").named(
            "clusters",
            SyntaxShape::String,
            "the clusters which should be contacted",
            None,
        )
    }

    fn usage(&self) -> &str {
        "Shows roles and domain for the connected user"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        whoami(self.state.clone(), args)
    }
}

fn whoami(state: Arc<State>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once()?;

    let cluster_identifiers = cluster_identifiers_from(&state, &args, true)?;

    let mut entries = vec![];
    for identifier in cluster_identifiers {
        let cluster = match state.clusters().get(&identifier) {
            Some(c) => c.cluster(),
            None => {
                return Err(ShellError::untagged_runtime_error("Cluster not found"));
            }
        };

        let response = block_on(cluster.management_request(ManagementRequest::Whoami));
        let mut content: Map<String, Value> = serde_json::from_str(response.content())?;
        content.insert("cluster".into(), json!(identifier.clone()));
        let converted = convert_json_value_to_nu_value(&Value::Object(content), Tag::default())?;
        entries.push(converted);
    }

    Ok(entries.into())
}
