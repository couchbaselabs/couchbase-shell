use crate::cli::clusters_register::update_config_file;
use crate::state::State;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_stream::OutputStream;
use std::sync::{Arc, Mutex};

pub struct ClustersUnregister {
    state: Arc<Mutex<State>>,
}

impl ClustersUnregister {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl nu_engine::WholeStreamCommand for ClustersUnregister {
    fn name(&self) -> &str {
        "clusters unregister"
    }

    fn signature(&self) -> Signature {
        Signature::build("clusters unregister")
            .required(
                "identifier",
                SyntaxShape::String,
                "the identifier to use for this cluster",
            )
            .switch(
                "save",
                "whether or not to add the cluster to the .cbsh config file, defaults to false",
                None,
            )
    }

    fn usage(&self) -> &str {
        "Registers a cluster for use with the shell"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        clusters_unregister(args, self.state.clone())
    }
}

fn clusters_unregister(
    args: CommandArgs,
    state: Arc<Mutex<State>>,
) -> Result<OutputStream, ShellError> {
    let identifier: String = args.req(0)?;
    let save = args.get_flag("save")?.unwrap_or(false);

    let mut guard = state.lock().unwrap();
    if guard.active() == identifier.clone() {
        return Err(ShellError::unexpected(
            "Cannot unregister the active cluster",
        ));
    }

    if guard.remove_cluster(identifier).is_none() {
        return Err(ShellError::unexpected(
            "identifier is not registered to a cluster",
        ));
    };

    if save {
        update_config_file(&mut guard)?;
    };

    Ok(OutputStream::empty())
}
