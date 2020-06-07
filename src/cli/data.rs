use async_stream::stream;
use async_trait::async_trait;
use nu_cli::{CommandArgs, CommandRegistry, OutputStream};
use nu_errors::ShellError;
use nu_protocol::{Signature, UntaggedValue};
use nu_source::Tag;

pub struct Data;

#[async_trait]
impl nu_cli::WholeStreamCommand for Data {
    fn name(&self) -> &str {
        "data"
    }

    fn signature(&self) -> Signature {
        Signature::build("data")
    }

    fn usage(&self) -> &str {
        "Performs operations against the data service"
    }

    async fn run(
        &self,
        _args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        let registry = registry.clone();
        let stream = stream! {
            yield UntaggedValue::string(nu_cli::get_help(&Data, &registry))
            .into_value(Tag::unknown())
        };
        Ok(OutputStream::from_input(stream))
    }
}