use crate::state::State;
use async_trait::async_trait;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, TaggedDictBuilder};
use nu_source::Tag;
use nu_stream::OutputStream;
use std::sync::{Arc, Mutex};

pub struct UseCloudControlPlane {
    state: Arc<Mutex<State>>,
}

impl UseCloudControlPlane {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for UseCloudControlPlane {
    fn name(&self) -> &str {
        "use cloud-control-plane"
    }

    fn signature(&self) -> Signature {
        Signature::build("use cloud-control-plane").required(
            "identifier",
            SyntaxShape::String,
            "the identifier of the cloud control plane",
        )
    }

    fn usage(&self) -> &str {
        "Sets the active cloud control plane based on its identifier"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let guard = self.state.lock().unwrap();
        guard.set_active_cloud_control_plane(args.req(0)?)?;

        let mut using_now = TaggedDictBuilder::new(Tag::default());
        using_now.insert_value(
            "cloud_control_plane",
            guard.active_cloud_control_plane_name().unwrap(),
        );
        let cloud = vec![using_now.into_value()];
        Ok(cloud.into())
    }
}
