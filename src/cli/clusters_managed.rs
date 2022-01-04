use crate::cli::util::cluster_identifiers_from;
use crate::state::State;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, TaggedDictBuilder, UntaggedValue};
use nu_source::Tag;
use nu_stream::OutputStream;
use std::sync::{Arc, Mutex};

pub struct ClustersManaged {
    state: Arc<Mutex<State>>,
}

impl ClustersManaged {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl nu_engine::WholeStreamCommand for ClustersManaged {
    fn name(&self) -> &str {
        "clusters managed"
    }

    fn signature(&self) -> Signature {
        Signature::build("clusters")
    }

    fn usage(&self) -> &str {
        "Lists all clusters currently managed by couchbase shell"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        clusters(args, self.state.clone())
    }
}

fn clusters(args: CommandArgs, state: Arc<Mutex<State>>) -> Result<OutputStream, ShellError> {
    let identifiers = cluster_identifiers_from(&state, &args, false)?;

    let guard = state.lock().unwrap();
    let active = guard.active();
    let clusters = guard
        .clusters()
        .iter()
        .filter(|(k, _)| identifiers.contains(k))
        .map(|(k, v)| {
            let mut collected = TaggedDictBuilder::new(Tag::default());
            collected.insert_untagged("active", UntaggedValue::boolean(k == &active));
            collected.insert_value("tls", UntaggedValue::boolean(v.tls_config().enabled()));
            collected.insert_value("identifier", k.clone());
            collected.insert_value("username", String::from(v.username()));
            collected.insert_value(
                "capella_organization",
                v.capella_org().unwrap_or_else(|| "".to_string()),
            );
            collected.into_value()
        })
        .collect::<Vec<_>>();

    Ok(clusters.into())
}
