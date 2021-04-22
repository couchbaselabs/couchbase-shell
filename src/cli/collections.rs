use async_trait::async_trait;
use nu_cli::ActionStream;
use nu_engine::{get_full_help, CommandArgs};
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue};
use nu_source::Tag;

pub struct Collections;

#[async_trait]
impl nu_engine::WholeStreamCommand for Collections {
    fn name(&self) -> &str {
        "collections"
    }

    fn signature(&self) -> Signature {
        Signature::build("collections")
    }

    fn usage(&self) -> &str {
        "Perform collection management operations"
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        Ok(ActionStream::one(ReturnSuccess::value(
            UntaggedValue::string(get_full_help(&Collections, &args.scope))
                .into_value(Tag::unknown()),
        )))
    }
}
