//! Command which delegates to the nu_plugin_fetch plugin so it's bundled with our binary.

use nu_engine::{CommandArgs, WholeStreamCommand};

use nu_errors::ShellError;
use nu_plugin::Plugin;
use nu_plugin_fetch::Fetch;
use nu_protocol::Signature;
use nu_stream::ActionStream;

pub struct PluginFetch {
    signature: Signature,
}

impl PluginFetch {
    pub fn new() -> Self {
        let mut original = Fetch::new();
        let signature = original.config().unwrap();
        Self { signature }
    }
}

impl WholeStreamCommand for PluginFetch {
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
        let call_info = args.call_info.clone();
        let call_info = call_info.evaluate(args.context())?;

        let mut original = Fetch::new();
        let result = original.begin_filter(call_info)?;

        Ok(ActionStream::new(result.into_iter()))
    }
}
