use async_trait::async_trait;
use nu_cli::{CommandArgs, OutputStream};
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue};
use nu_source::Tag;

pub struct Doc;

#[async_trait]
impl nu_cli::WholeStreamCommand for Doc {
    fn name(&self) -> &str {
        "doc"
    }

    fn signature(&self) -> Signature {
        Signature::build("doc")
    }

    fn usage(&self) -> &str {
        "Perform document operations against a bucket or collection"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        Ok(OutputStream::one(ReturnSuccess::value(
            UntaggedValue::string(nu_cli::get_help(&Doc, &args.scope)).into_value(Tag::unknown()),
        )))
    }
}
