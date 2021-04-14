use crate::state::State;
use async_trait::async_trait;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, TaggedDictBuilder};
use nu_source::Tag;
use nu_stream::OutputStream;
use std::sync::Arc;

pub struct UseBucket {
    state: Arc<State>,
}

impl UseBucket {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for UseBucket {
    fn name(&self) -> &str {
        "use bucket"
    }

    fn signature(&self) -> Signature {
        Signature::build("use bucket").required(
            "identifier",
            SyntaxShape::String,
            "the name of the bucket",
        )
    }

    fn usage(&self) -> &str {
        "Sets the active bucket based on its name"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let args = args.evaluate_once()?;

        let active = self.state.active_cluster();

        if let Some(id) = args.nth(0) {
            active.set_active_bucket(id.as_string()?);
        }

        let mut using_now = TaggedDictBuilder::new(Tag::default());
        using_now.insert_value(
            "bucket",
            active
                .active_bucket()
                .map(|s| s.clone())
                .unwrap_or(String::from("<not set>")),
        );
        let clusters = vec![using_now.into_value()];
        Ok(clusters.into())
    }
}
