use crate::client::ManagementRequest;
use crate::state::State;
use async_trait::async_trait;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, UntaggedValue};
use nu_source::Tag;
use nu_stream::OutputStream;
use std::sync::Arc;

pub struct Tutorial {
    state: Arc<State>,
}

impl Tutorial {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for Tutorial {
    fn name(&self) -> &str {
        "tutorial"
    }

    fn signature(&self) -> Signature {
        Signature::build("tutorial")
    }

    fn usage(&self) -> &str {
        "Run the Couchbase Shell tutorial"
    }

    fn run(&self, _args: CommandArgs) -> Result<OutputStream, ShellError> {
        run_tutorial(self.state.clone())
    }
}

fn run_tutorial(state: Arc<State>) -> Result<OutputStream, ShellError> {
    let tutorial = state.tutorial();
    let cluster = state.active_cluster().cluster();
    let resp = cluster.management_request(ManagementRequest::GetBucket {
        name: "travel-sample".into(),
    })?;

    let exists = match resp.status() {
        200 => true,
        _ => false,
    };

    Ok(OutputStream::one(
        UntaggedValue::string(tutorial.current_step(exists)).into_value(Tag::unknown()),
    ))
}
