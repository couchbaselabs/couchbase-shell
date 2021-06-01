use crate::cli::util::cluster_identifiers_from;
use crate::state::State;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue};
use nu_source::Tag;
use nu_stream::OutputStream;
use std::sync::{Arc, Mutex};

pub struct Clusters {
    state: Arc<Mutex<State>>,
}

impl Clusters {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl nu_engine::WholeStreamCommand for Clusters {
    fn name(&self) -> &str {
        "clusters"
    }

    fn signature(&self) -> Signature {
        Signature::build("clusters").named(
            "clusters",
            SyntaxShape::String,
            "the clusters which should be contacted",
            None,
        )
    }

    fn usage(&self) -> &str {
        "Lists all managed clusters"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        clusters(args, self.state.clone())
    }
}

fn clusters(args: CommandArgs, state: Arc<Mutex<State>>) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once()?;

    let identifiers = cluster_identifiers_from(&state, &args, false)?;

    let active = state.lock().unwrap().active();
    let clusters = state
        .lock()
        .unwrap()
        .clusters()
        .iter()
        .filter(|(k, _)| identifiers.contains(k))
        .map(|(k, v)| {
            let mut collected = TaggedDictBuilder::new(Tag::default());
            collected.insert_untagged("active", UntaggedValue::boolean(k == &active));
            collected.insert_value("tls", UntaggedValue::boolean(v.tls_config().enabled()));
            collected.insert_value("identifier", k.clone());
            collected.insert_value("username", String::from(v.username()));
            collected.into_value()
        })
        .collect::<Vec<_>>();

    Ok(clusters.into())
}
