use crate::state::State;
use async_trait::async_trait;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, TaggedDictBuilder};
use nu_source::Tag;
use nu_stream::OutputStream;
use std::sync::Arc;

pub struct UseCollection {
    state: Arc<State>,
}

impl UseCollection {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for UseCollection {
    fn name(&self) -> &str {
        "use collection"
    }

    fn signature(&self) -> Signature {
        Signature::build("use collection").required(
            "identifier",
            SyntaxShape::String,
            "the name of the collection",
        )
    }

    fn usage(&self) -> &str {
        "Sets the active collection based on its name"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let args = args.evaluate_once()?;

        let active = self.state.active_cluster();

        if active.active_bucket().is_none() {
            return Err(ShellError::untagged_runtime_error(
                "You must select a bucket before a collection",
            ));
        }

        if let Some(id) = args.nth(0) {
            active.set_active_collection(id.as_string()?);
        }

        let mut using_now = TaggedDictBuilder::new(Tag::default());
        using_now.insert_value(
            "collection",
            active
                .active_collection()
                .map(|s| s.clone())
                .unwrap_or(String::from("<not set>")),
        );
        let clusters = vec![using_now.into_value()];
        Ok(clusters.into())
    }
}
