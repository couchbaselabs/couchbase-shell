use super::util::convert_json_value_to_nu_value;
use crate::cli::convert_cb_error;
use crate::cli::util::cluster_identifiers_from;
use crate::state::State;
use couchbase::{GenericManagementRequest, Request};
use futures::channel::oneshot;
use nu_cli::{CommandArgs, CommandRegistry, OutputStream};
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_source::Tag;
use serde_json::{json, Map, Value};
use std::sync::Arc;
use async_trait::async_trait;

pub struct Whoami {
    state: Arc<State>,
}

impl Whoami {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_cli::WholeStreamCommand for Whoami {
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

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        whoami(self.state.clone(), args, registry).await
    }
}

async fn whoami(
    state: Arc<State>,
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry).await?;

    let identifier_arg = args
        .get("clusters")
        .map(|id| id.as_string().unwrap())
        .unwrap_or_else(|| state.active());

    let cluster_identifiers = cluster_identifiers_from(&state, identifier_arg.as_str());

    let mut entries = vec![];
    for identifier in cluster_identifiers {
        let core = state.clusters().get(&identifier).unwrap().cluster().core();

        let (sender, receiver) = oneshot::channel();
        let request = GenericManagementRequest::new(sender, "/whoami".into(), "get".into(), None);
        core.send(Request::GenericManagementRequest(request));

        let result = convert_cb_error(receiver.await.unwrap())?;

        if result.payload().is_none() {
            return Err(ShellError::untagged_runtime_error(
                "Empty response from cluster even though got 200 ok",
            ));
        }

        let mut resp: Map<String, Value> =
            serde_json::from_slice(result.payload().unwrap()).unwrap();
        resp.insert("cluster".into(), json!(identifier.clone()));
        let converted = convert_json_value_to_nu_value(&Value::Object(resp), Tag::default());

        entries.push(converted);
    }

    Ok(entries.into())
}
