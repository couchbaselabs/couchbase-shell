use crate::cli::util::NuValueMap;
use nu_engine::command_prelude::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature};

#[derive(Clone)]
pub struct Version;

impl Command for Version {
    fn name(&self) -> &str {
        "version"
    }

    fn signature(&self) -> Signature {
        Signature::build("version").category(Category::Custom("couchbase".to_string()))
    }

    fn description(&self) -> &str {
        "The cbsh version"
    }

    fn run(
        &self,
        _engine_state: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let mut collected = NuValueMap::default();
        collected.add_string(
            "version",
            option_env!("CARGO_PKG_VERSION").unwrap_or("0.0.0"),
            call.head,
        );

        Ok(collected.into_pipeline_data(call.head))
    }
}
