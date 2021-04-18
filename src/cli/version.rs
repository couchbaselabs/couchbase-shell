use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, TaggedDictBuilder};
use nu_source::Tag;
use nu_stream::OutputStream;

pub struct Version {}

impl Version {
    pub fn new() -> Self {
        Self {}
    }
}

impl nu_engine::WholeStreamCommand for Version {
    fn name(&self) -> &str {
        "version"
    }

    fn signature(&self) -> Signature {
        Signature::build("version")
    }

    fn usage(&self) -> &str {
        "The cbsh version"
    }

    fn run(&self, _args: CommandArgs) -> Result<OutputStream, ShellError> {
        let mut collected = TaggedDictBuilder::new(Tag::default());
        collected.insert_value(
            "version",
            option_env!("CARGO_PKG_VERSION").unwrap_or("0.0.0"),
        );
        let result = collected.into_value();

        Ok(vec![result].into())
    }
}
