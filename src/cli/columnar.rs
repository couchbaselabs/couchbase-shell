use nu_engine::command_prelude::Call;
use nu_engine::get_full_help;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, IntoPipelineData, PipelineData, ShellError, Signature, Value};

#[derive(Clone)]
pub struct Columnar;

impl Command for Columnar {
    fn name(&self) -> &str {
        "columnar"
    }

    fn signature(&self) -> Signature {
        Signature::build("columnar").category(Category::Custom("couchbase".to_string()))
    }

    fn description(&self) -> &str {
        "Perform mangement operations against Columnar analytics clusters"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(
            Value::string(get_full_help(&Columnar, engine_state, stack), call.head)
                .into_pipeline_data(),
        )
    }
}
