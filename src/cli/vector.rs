use nu_engine::get_full_help;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, IntoPipelineData, PipelineData, ShellError, Signature, Value};

#[derive(Clone)]
pub struct Vector;

impl Command for Vector {
    fn name(&self) -> &str {
        "vector"
    }

    fn signature(&self) -> Signature {
        Signature::build("vector").category(Category::Custom("couchbase".to_string()))
    }

    fn usage(&self) -> &str {
        "Commands that support and implement vector search"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(Value::String {
            val: get_full_help(&Vector, engine_state, stack),
            internal_span: call.head,
        }
        .into_pipeline_data())
    }
}
