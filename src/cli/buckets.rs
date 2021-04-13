use nu_cli::ActionStream;
use nu_engine::{get_full_help, CommandArgs};
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue};
use nu_source::Tag;

pub struct Buckets;

impl nu_engine::WholeStreamCommand for Buckets {
    fn name(&self) -> &str {
        "buckets"
    }

    fn signature(&self) -> Signature {
        Signature::build("buckets")
    }

    fn usage(&self) -> &str {
        "Perform bucket management operations"
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        Ok(ActionStream::one(ReturnSuccess::value(
            UntaggedValue::string(get_full_help(&Buckets, &args.scope)).into_value(Tag::unknown()),
        )))
    }
}
