use async_trait::async_trait;
use nu_cli::{CommandArgs, OutputStream};
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, UntaggedValue};
use nu_source::Tag;

pub struct Scopes;

#[async_trait]
impl nu_cli::WholeStreamCommand for Scopes {
    fn name(&self) -> &str {
        "scopes"
    }

    fn signature(&self) -> Signature {
        Signature::build("scopes")
    }

    fn usage(&self) -> &str {
        "Perform scope management operations"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        Ok(OutputStream::one(ReturnSuccess::value(
            UntaggedValue::string(nu_cli::get_help(&Scopes, &args.scope))
                .into_value(Tag::unknown()),
        )))
    }
}
