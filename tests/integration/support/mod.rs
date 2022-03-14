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

#[cfg(windows)]
pub const NATIVE_PATH_ENV_SEPARATOR: char = ';';
#[cfg(not(windows))]
pub const NATIVE_PATH_ENV_SEPARATOR: char = ':';

impl Outcome {
    pub fn new(out: String, err: String) -> Outcome {
        Outcome { out, err }
    }
}

pub fn pipeline(commands: &str) -> String {
    commands
        .trim()
        .lines()
        .map(|line| line.trim())
        .collect::<Vec<&str>>()
        .join(" ")
        .trim_end()
        .to_string()
}

pub fn shell_os_paths() -> Vec<std::path::PathBuf> {
    let mut original_paths = vec![];

    if let Some(paths) = std::env::var_os(NATIVE_PATH_ENV_VAR) {
        original_paths = std::env::split_paths(&paths).collect::<Vec<_>>();
    }

    original_paths
}
