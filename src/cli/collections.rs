use async_trait::async_trait;
use nu_cli::OutputStream;
use nu_engine::CommandArgs;
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

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        Ok(OutputStream::one(ReturnSuccess::value(
            UntaggedValue::string(nu_engine::get_help(&Collections, &args.scope))
                .into_value(Tag::unknown()),
        )))
    }
}
