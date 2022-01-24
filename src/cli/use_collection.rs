use crate::state::State;
use async_trait::async_trait;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, TaggedDictBuilder};
use nu_source::Tag;
use nu_stream::OutputStream;
use std::sync::{Arc, Mutex};

pub struct UseCollection {
    state: Arc<Mutex<State>>,
}

impl UseCollection {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
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
        let guard = self.state.lock().unwrap();
        let active = match guard.active_cluster() {
            Some(c) => c,
            None => {
                return Err(ShellError::unexpected("An active cluster must be set"));
            }
        };

        if active.active_bucket().is_none() {
            return Err(ShellError::unexpected(
                "You must select a bucket before a collection",
            ));
        }

        active.set_active_collection(args.req(0)?);

        let mut using_now = TaggedDictBuilder::new(Tag::default());
        using_now.insert_value(
            "collection",
            active
                .active_collection()
                .unwrap_or_else(|| String::from("<not set>")),
        );
        let clusters = vec![using_now.into_value()];
        Ok(clusters.into())
    }
}
