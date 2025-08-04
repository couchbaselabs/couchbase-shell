use async_trait::async_trait;
use nu_cli::OutputStream;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue};
use nu_source::Tag;

pub struct Data;

#[async_trait]
impl nu_engine::WholeStreamCommand for Data {
    fn name(&self) -> &str {
        "data"
    }

    fn signature(&self) -> Signature {
        Signature::build("data")
    }

    fn usage(&self) -> &str {
        "Performs operations against the data service"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        Ok(OutputStream::one(ReturnSuccess::value(
            UntaggedValue::string(nu_engine::get_help(&Data, &args.scope))
                .into_value(Tag::unknown()),
        )))
    }
}
