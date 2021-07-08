use super::util::convert_json_value_to_nu_value;
use crate::cli::util::{cluster_identifiers_from, validate_is_not_cloud};
use crate::client::ManagementRequest;
use crate::state::State;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_source::Tag;
use nu_stream::OutputStream;
use serde_json::{json, Map, Value};
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

pub struct Whoami {
    state: Arc<Mutex<State>>,
}

impl Whoami {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

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

fn whoami(state: Arc<Mutex<State>>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();

    let cluster_identifiers = cluster_identifiers_from(&state, &args, true)?;

    let mut entries = vec![];
    for identifier in cluster_identifiers {
        let guard = state.lock().unwrap();
        let cluster = match guard.clusters().get(&identifier) {
            Some(c) => c,
            None => {
                return Err(ShellError::untagged_runtime_error("Cluster not found"));
            }
        };
        validate_is_not_cloud(cluster, "whoami cannot be run against cloud clusters")?;

        let response = cluster.cluster().http_client().management_request(
            ManagementRequest::Whoami,
            Instant::now().add(cluster.timeouts().management_timeout()),
            ctrl_c.clone(),
        )?;
        let mut content: Map<String, Value> = serde_json::from_str(response.content())?;
        content.insert("cluster".into(), json!(identifier.clone()));
        let converted = convert_json_value_to_nu_value(&Value::Object(content), Tag::default())?;
        entries.push(converted);
    }

    Ok(entries.into())
}
