use crate::state::State;
use async_trait::async_trait;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, TaggedDictBuilder};
use nu_source::Tag;
use nu_stream::OutputStream;
use std::sync::Arc;

pub struct UseScope {
    state: Arc<State>,
}

impl UseScope {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for UseScope {
    fn name(&self) -> &str {
        "use scope"
    }

    fn signature(&self) -> Signature {
        Signature::build("use scope").required(
            "identifier",
            SyntaxShape::String,
            "the name of the scope",
        )
    }

    fn usage(&self) -> &str {
        "Sets the active scope based on its name"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let args = args.evaluate_once()?;

        let active = self.state.active_cluster();

        if active.active_bucket().is_none() {
            return Err(ShellError::untagged_runtime_error(
                "You must select a bucket before a scope",
            ));
        }

        if let Some(id) = args.nth(0) {
            active.set_active_scope(id.as_string()?);
        }

        let mut using_now = TaggedDictBuilder::new(Tag::default());
        using_now.insert_value(
            "scope",
            active
                .active_scope()
                .unwrap_or_else(|| String::from("<not set>")),
        );
        let clusters = vec![using_now.into_value()];
        Ok(clusters.into())
    }
}
