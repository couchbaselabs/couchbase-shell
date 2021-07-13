use crate::state::State;
use async_trait::async_trait;

use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, TaggedDictBuilder, UntaggedValue};
use nu_source::Tag;
use nu_stream::OutputStream;

use std::sync::{Arc, Mutex};

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
        Signature::build("clouds").switch("all", "List all clouds", None)
    }

    fn usage(&self) -> &str {
        "Lists all managed clouds"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        clouds(self.state.clone(), args)
    }
}

fn clouds(state: Arc<Mutex<State>>, _args: CommandArgs) -> Result<OutputStream, ShellError> {
    let guard = state.lock().unwrap();
    let active_cloud = guard.active_cloud_name().unwrap_or_else(|| "".to_string());
    let mut results = vec![];
    for cloud in guard.clouds() {
        let mut collected = TaggedDictBuilder::new(Tag::default());
        collected.insert_untagged(
            "active",
            UntaggedValue::boolean(cloud.0.clone() == active_cloud.clone()),
        );
        collected.insert_value("identifier", cloud.0.clone());
        results.push(collected.into_value())
    }

    Ok(OutputStream::from(results))
}
