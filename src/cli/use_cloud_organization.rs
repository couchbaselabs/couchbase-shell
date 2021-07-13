use crate::state::State;
use async_trait::async_trait;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, TaggedDictBuilder};
use nu_source::Tag;
use nu_stream::OutputStream;
use std::sync::{Arc, Mutex};

pub struct UseCloudOrganization {
    state: Arc<Mutex<State>>,
}

impl UseCloudOrganization {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for UseCloudOrganization {
    fn name(&self) -> &str {
        "use cloud-organization"
    }

    fn signature(&self) -> Signature {
        Signature::build("use cloud-organization").required(
            "identifier",
            SyntaxShape::String,
            "the identifier of the cloud organization",
        )
    }

    fn usage(&self) -> &str {
        "Sets the active cloud organization based on its identifier"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let guard = self.state.lock().unwrap();
        guard.set_active_cloud_org(args.req(0)?)?;

        let mut using_now = TaggedDictBuilder::new(Tag::default());
        using_now.insert_value("cloud_organization", guard.active_cloud_org_name().unwrap());
        let cloud = vec![using_now.into_value()];
        Ok(cloud.into())
    }
}
