use crate::ShellConfig;
use nu_cli::eval_config_contents;
use nu_path::canonicalize_with;
use nu_protocol::engine::{EngineState, Stack, StateWorkingSet};
use std::path::PathBuf;

pub(crate) const CBSHELL_FOLDER: &str = "cbshell";
const HISTORY_FILE: &str = "history.txt";

pub(crate) fn read_nu_config_file(
    engine_state: &mut EngineState,
    stack: &mut Stack,
    config_file: PathBuf,
) {
    let working_set = StateWorkingSet::new(engine_state);
    let cwd = working_set.get_cwd();

    let path = canonicalize_with(&config_file, cwd).expect("Failed to find config file");
    eval_config_contents(path, engine_state, stack);
}

pub(crate) fn create_history_path(config: ShellConfig) -> Option<PathBuf> {
    let history_path = if let Some(p) = config.location() {
        let mut p = p.clone();
        p.pop();
        p.push(HISTORY_FILE);
        p
    } else {
        "".into()
    };

    if !history_path.exists() {
        // Creating an empty file to store the history
        match std::fs::File::create(&history_path) {
            Ok(_) => Some(history_path),
            Err(_) => None,
        }
    } else {
        Some(history_path)
    }
}
