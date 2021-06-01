use crate::state::State;
use async_trait::async_trait;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, TaggedDictBuilder};
use nu_source::Tag;
use nu_stream::OutputStream;
use std::sync::{Arc, Mutex};

pub struct UseCluster {
    state: Arc<Mutex<State>>,
}

impl UseCluster {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for UseCluster {
    fn name(&self) -> &str {
        "use cluster"
    }

    fn signature(&self) -> Signature {
        Signature::build("use cluster").required(
            "identifier",
            SyntaxShape::String,
            "the identifier of the cluster",
        )
    }

    fn usage(&self) -> &str {
        "Sets the active cluster based on its identifier"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let args = args.evaluate_once()?;

        let guard = self.state.lock().unwrap();
        if let Some(id) = args.nth(0) {
            match guard.set_active(id.as_string()?) {
                Ok(v) => v,
                Err(_) => {
                    return Err(ShellError::untagged_runtime_error(
                        "Could not set active cluster",
                    ));
                }
            }
        }

        let mut using_now = TaggedDictBuilder::new(Tag::default());
        using_now.insert_value("cluster", guard.active());
        let clusters = vec![using_now.into_value()];
        Ok(clusters.into())
    }
}
