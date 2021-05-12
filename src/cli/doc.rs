use async_trait::async_trait;
use nu_cli::ActionStream;
use nu_engine::{get_full_help, CommandArgs};
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue};
use nu_source::Tag;

pub struct Doc;

#[async_trait]
impl nu_engine::WholeStreamCommand for Doc {
    fn name(&self) -> &str {
        "doc"
    }

    fn signature(&self) -> Signature {
        Signature::build("doc")
    }

    fn usage(&self) -> &str {
        "Perform document operations against a bucket or collection"
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        Ok(ActionStream::one(ReturnSuccess::value(
            UntaggedValue::string(get_full_help(&Doc, args.scope())).into_value(Tag::unknown()),
        )))
    }
}
