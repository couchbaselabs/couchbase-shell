use nu_engine::{get_full_help, CommandArgs};
use nu_errors::ShellError;
use nu_protocol::{Signature, UntaggedValue};
use nu_source::Tag;
use nu_stream::OutputStream;

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

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        Ok(OutputStream::one(
            UntaggedValue::string(get_full_help(&Buckets, args.scope())).into_value(Tag::unknown()),
        ))
    }
}
