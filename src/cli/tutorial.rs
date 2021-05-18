use crate::client::ManagementRequest;
use crate::state::State;
use async_trait::async_trait;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, UntaggedValue};
use nu_source::Tag;
use nu_stream::OutputStream;
use std::ops::Add;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::time::Instant;

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

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let ctrl_c = args.ctrl_c();
        run_tutorial(self.state.clone(), ctrl_c.clone())
    }
}

fn run_tutorial(state: Arc<State>, ctrl_c: Arc<AtomicBool>) -> Result<OutputStream, ShellError> {
    let tutorial = state.tutorial();
    let active_cluster = state.active_cluster();
    let resp = active_cluster.cluster().management_request(
        ManagementRequest::GetBucket {
            name: "travel-sample".into(),
        },
        Instant::now().add(active_cluster.timeouts().query_timeout()),
        ctrl_c.clone(),
    );

    let exists = match resp {
        Ok(r) => match r.status() {
            200 => true,
            _ => false,
        },
        Err(_) => false,
    };

    Ok(OutputStream::one(
        UntaggedValue::string(tutorial.current_step(exists)).into_value(Tag::unknown()),
    ))
}
