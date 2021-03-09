use crate::cli::util::read_file_from_home;
use async_trait::async_trait;
use log::debug;
use nu_cli::OutputStream;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, TaggedDictBuilder};
use nu_source::Tag;
use std::io::Read;

pub struct SDKLog;

#[async_trait]
impl nu_engine::WholeStreamCommand for SDKLog {
    fn name(&self) -> &str {
        "sdklog"
    }

    fn signature(&self) -> Signature {
        Signature::build("sdklog").named("last", SyntaxShape::Int, "How many lines to print", None)
    }

    fn usage(&self) -> &str {
        "Print the last x lines from the sdk log"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        sdk_log(args).await
    }
}

async fn sdk_log(args: CommandArgs) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once().await?;

    let last = match args.get("last") {
        Some(v) => match v.as_u64() {
            Ok(l) => l,
            Err(e) => return Err(e),
        },
        None => 10,
    };

    debug!("Fetching last {} lines from sdk log", &last);

    let mut file = read_file_from_home("sdk.log".into())?;

    let mut text = String::new();
    match file.read_to_string(&mut text) {
        Ok(_) => {}
        Err(e) => {
            return Err(ShellError::untagged_runtime_error(format!(
                "Failed to read log file {}",
                e
            )))
        }
    };

    let mut results = vec![];
    for line in text.lines().rev() {
        let mut collected = TaggedDictBuilder::new(Tag::default());
        collected.insert_value("logs", line);
        results.push(collected.into_value());
        if results.len() as u64 == last {
            break;
        }
    }

    Ok(OutputStream::from(results))
}
