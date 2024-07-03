use nu_engine::get_full_help;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, IntoPipelineData, PipelineData, ShellError, Signature, Value};

#[derive(Clone)]
pub struct Doc;

impl Command for Doc {
    fn name(&self) -> &str {
        "doc"
    }

    fn signature(&self) -> Signature {
        Signature::build("doc").category(Category::Custom("couchbase".to_string()))
    }

    fn usage(&self) -> &str {
        "Perform document operations against a bucket or collection"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(Value::String {
            val: get_full_help(
                &Doc,
                engine_state,
                stack,
            ),
            internal_span: call.head,
        }
        .into_pipeline_data())
    }
}
