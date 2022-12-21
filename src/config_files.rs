use log::{debug, warn};
use nu_cli::eval_config_contents;
use nu_protocol::engine::{EngineState, Stack};
use std::fs;
use std::fs::File;
use std::io::Write;

pub(crate) const CBSHELL_FOLDER: &str = "CouchbaseShell";
const CONFIG_FILE: &str = "config.nu";

pub(crate) fn read_nu_config_file(engine_state: &mut EngineState, stack: &mut Stack) {
    if let Some(mut config_path) = nu_path::config_dir() {
        config_path.push(CBSHELL_FOLDER);

        if !config_path.exists() {
            if let Err(err) = std::fs::create_dir_all(&config_path) {
                eprintln!("Failed to create config directory: {}", err);
                return;
            }
        }

        config_path.push(CONFIG_FILE);

        // Until we have some sort of versioning we need to remove this config file always so that
        // any updated default config file is used.
        if config_path.exists() {
            debug!(
                "Config file found at {}, removing",
                config_path.to_string_lossy()
            );
            match fs::remove_file(&config_path) {
                Ok(()) => {}
                Err(e) => warn!("Failed to remove existing config file: {}", e),
            };
        } else {
            debug!("No config file found at {}", config_path.to_string_lossy());
        }

        let config_file = if cfg!(windows) {
            include_str!("../docs/sample_config/default_config_windows.nu")
        } else {
            include_str!("../docs/sample_config/default_config.nu")
        };

        let mut output = File::create(&config_path).expect("Unable to create file");
        write!(output, "{}", config_file).expect("Unable to write to config file");
        debug!("Config file created at: {}", config_path.to_string_lossy());

        eval_config_contents(config_path, engine_state, stack);
    }
}
