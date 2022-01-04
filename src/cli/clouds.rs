use crate::state::State;
use async_trait::async_trait;

use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, TaggedDictBuilder};
use nu_source::Tag;
use nu_stream::OutputStream;

use crate::cli::cloud_json::JSONCloudsResponse;
use crate::client::CapellaRequest;
use log::debug;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

pub struct Clouds {
    state: Arc<Mutex<State>>,
}

impl Clouds {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for Clouds {
    fn name(&self) -> &str {
        "clouds"
    }

    fn signature(&self) -> Signature {
        Signature::build("clouds").named(
            "capella",
            SyntaxShape::String,
            "the Capella organization to use",
            None,
        )
    }

    fn usage(&self) -> &str {
        "Shows the current status for all clouds belonging to the active Capella organization"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        clouds(self.state.clone(), args)
    }
}

fn clouds(state: Arc<Mutex<State>>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();
    let capella = args.get_flag("capella")?;

    debug!("Running clouds");

    let guard = state.lock().unwrap();
    let control = if let Some(c) = capella {
        guard.capella_org_for_cluster(c)
    } else {
        guard.active_capella_org()
    }?;
    let client = control.client();
    let response = client.capella_request(
        CapellaRequest::GetClouds {},
        Instant::now().add(control.timeout()),
        ctrl_c,
    )?;
    if response.status() != 200 {
        return Err(ShellError::unexpected(response.content().to_string()));
    };

    let content: JSONCloudsResponse = serde_json::from_str(response.content())?;

    let mut results = vec![];
    for cloud in content.items().into_iter() {
        let mut collected = TaggedDictBuilder::new(Tag::default());
        collected.insert_value("identifier", cloud.name());
        collected.insert_value("status", cloud.status());
        collected.insert_value("region", cloud.region());
        collected.insert_value("provider", cloud.provider());
        collected.insert_value("cloud_id", cloud.id());
        results.push(collected.into_value())
    }

    Ok(OutputStream::from(results))
}
