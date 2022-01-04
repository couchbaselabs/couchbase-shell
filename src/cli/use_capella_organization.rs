use crate::state::State;
use async_trait::async_trait;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, TaggedDictBuilder};
use nu_source::Tag;
use nu_stream::OutputStream;
use std::sync::{Arc, Mutex};

pub struct UseCapellaOrganization {
    state: Arc<Mutex<State>>,
}

impl UseCapellaOrganization {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for UseCapellaOrganization {
    fn name(&self) -> &str {
        "use capella-organization"
    }

    fn signature(&self) -> Signature {
        Signature::build("use capella-organization").required(
            "identifier",
            SyntaxShape::String,
            "the identifier of the capella organization",
        )
    }

    fn usage(&self) -> &str {
        "Sets the active capella organization based on its identifier"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let guard = self.state.lock().unwrap();
        guard.set_active_capella_org(args.req(0)?)?;

        let mut using_now = TaggedDictBuilder::new(Tag::default());
        using_now.insert_value(
            "capella_organization",
            guard.active_capella_org_name().unwrap(),
        );
        let cloud = vec![using_now.into_value()];
        Ok(cloud.into())
    }
}
