//! Command which delegates to the nu_plugin_from_bson plugin so it's bundled with our binary.

use nu_engine::{CommandArgs, WholeStreamCommand};

use nu_errors::ShellError;
use nu_plugin::Plugin;
use nu_plugin_from_bson::FromBson;
use nu_protocol::Signature;
use nu_stream::ActionStream;

pub struct PluginFromBson {
    signature: Signature,
}

impl PluginFromBson {
    pub fn new() -> Self {
        let mut original = FromBson::new();
        let signature = original.config().unwrap();
        Self { signature }
    }
}

impl WholeStreamCommand for PluginFromBson {
    fn name(&self) -> &str {
        self.signature.name.as_str()
    }

    fn usage(&self) -> &str {
        self.signature.usage.as_str()
    }

    fn signature(&self) -> Signature {
        self.signature.clone()
    }

    fn run_with_actions(&self, args: CommandArgs) -> Result<ActionStream, ShellError> {
        let mut original = FromBson::new();

        for input in args.input {
            original.filter(input)?;
        }

        Ok(ActionStream::new(original.end_filter()?.into_iter()))
    }
}
