use nu_cli::{CommandArgs, CommandRegistry, OutputStream};
use nu_errors::ShellError;
use nu_protocol::Signature;

pub struct Kv;

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

    fn run(
        &self,
        _args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        Ok(
            nu_cli::commands::help::get_help(self.name(), self.usage(), self.signature(), registry)
                .into(),
        )
    }
}
