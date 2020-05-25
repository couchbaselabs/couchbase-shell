use futures::executor::block_on;
use nu_cli::{CommandArgs, CommandRegistry, OutputStream};
use nu_errors::ShellError;
use nu_protocol::{Signature, TaggedDictBuilder};
use nu_source::Tag;

pub struct Version {}

impl Version {
    pub fn new() -> Self {
        Self {}
    }
}

impl nu_cli::WholeStreamCommand for Version {
    fn name(&self) -> &str {
        "version"
    }

    fn signature(&self) -> Signature {
        Signature::build("version")
    }

    fn usage(&self) -> &str {
        "The cbsh version"
    }

    fn run(
        &self,
        _args: CommandArgs,
        _registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        block_on(version())
    }
}

async fn version() -> Result<OutputStream, ShellError> {
    let mut collected = TaggedDictBuilder::new(Tag::default());
    collected.insert_value(
        "version",
        option_env!("CARGO_PKG_VERSION").unwrap_or("0.0.0"),
    );
    let result = collected.into_value();

    Ok(vec![result].into())
}
