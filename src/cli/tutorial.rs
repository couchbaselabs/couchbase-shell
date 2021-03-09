use crate::state::State;
use async_trait::async_trait;
use nu_cli::OutputStream;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::Signature;
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
    println!("{}", tutorial.current_step());

    Ok(OutputStream::empty())
}
