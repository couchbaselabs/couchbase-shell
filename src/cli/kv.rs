use async_stream::stream;
use nu_cli::{CommandArgs, CommandRegistry, OutputStream};
use nu_errors::ShellError;
use nu_protocol::{Signature, UntaggedValue};
use nu_source::Tag;
use async_trait::async_trait;

pub struct Kv;

#[async_trait]
impl nu_cli::WholeStreamCommand for Kv {
    fn name(&self) -> &str {
        "kv"
    }

    fn signature(&self) -> Signature {
        Signature::build("kv")
    }

    fn usage(&self) -> &str {
        "Perform Key/Value operations against a bucket"
    }

    async fn run(
        &self,
        _args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        let registry = registry.clone();
        let stream = stream! {
            yield UntaggedValue::string(nu_cli::get_help(&Kv, &registry))
            .into_value(Tag::unknown())
        };
        Ok(OutputStream::from_input(stream))
    }
}
