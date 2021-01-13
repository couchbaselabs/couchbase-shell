use async_trait::async_trait;
use nu_cli::{CommandArgs, OutputStream};
use nu_errors::ShellError;
use nu_protocol::{Signature, TaggedDictBuilder};
use nu_source::Tag;

pub struct Version {}

impl Version {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
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

    async fn run(&self, _args: CommandArgs) -> Result<OutputStream, ShellError> {
        version().await
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
