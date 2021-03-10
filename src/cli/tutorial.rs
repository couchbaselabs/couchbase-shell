use crate::state::State;
use async_trait::async_trait;
use couchbase::GetBucketOptions;
use nu_cli::OutputStream;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue};
use nu_source::Tag;
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

    async fn run(&self, _args: CommandArgs) -> Result<OutputStream, ShellError> {
        run_tutorial(self.state.clone()).await
    }
}

async fn run_tutorial(state: Arc<State>) -> Result<OutputStream, ShellError> {
    let tutorial = state.tutorial();
    let cluster = state.active_cluster().cluster();
    let mgr = cluster.buckets();
    let input = mgr
        .get_bucket("travel-sample", GetBucketOptions::default())
        .await;
    let exists = match input {
        Ok(_) => true,
        Err(_) => false,
    };

    Ok(OutputStream::one(ReturnSuccess::value(
        UntaggedValue::string(tutorial.current_step(exists)).into_value(Tag::unknown()),
    )))
}
