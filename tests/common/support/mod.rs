use nu_test_support::pipeline;

pub const LOGGER_PREFIX: &str = "[TEST]";

pub mod fs;
pub mod macros;

pub struct Outcome {
    pub out: String,
    pub err: String,
}

#[cfg(windows)]
pub const NATIVE_PATH_ENV_VAR: &str = "Path";
#[cfg(not(windows))]
pub const NATIVE_PATH_ENV_VAR: &str = "PATH";

impl Outcome {
    pub fn new(out: String, err: String) -> Outcome {
        Outcome { out, err }
    }
}

#[allow(dead_code)]
pub fn cb_pipeline(commands: impl Into<String>) -> String {
    pipeline(commands.into().as_str())
}

pub fn shell_os_paths() -> Vec<std::path::PathBuf> {
    let mut original_paths = vec![];

    if let Some(paths) = std::env::var_os(NATIVE_PATH_ENV_VAR) {
        original_paths = std::env::split_paths(&paths).collect::<Vec<_>>();
    }

    original_paths
}
