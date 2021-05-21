use async_trait::async_trait;
use nu_engine::{get_full_help, CommandArgs};
use nu_errors::ShellError;
use nu_protocol::{Signature, UntaggedValue};
use nu_source::Tag;
use nu_stream::OutputStream;

pub struct Scopes;

#[async_trait]
impl nu_engine::WholeStreamCommand for Scopes {
    fn name(&self) -> &str {
        "scopes"
    }

    fn signature(&self) -> Signature {
        Signature::build("scopes")
    }

    fn usage(&self) -> &str {
        "Perform scope management operations"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        Ok(OutputStream::one(
            UntaggedValue::string(get_full_help(&Scopes, args.scope())).into_value(Tag::unknown()),
        ))
    }
}
