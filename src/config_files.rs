use nu_cli::{eval_config_contents, eval_source};
use nu_protocol::engine::{EngineState, Stack};
use nu_protocol::{PipelineData, Span};
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

        if !config_path.exists() {
            println!("No config file found at {}", config_path.to_string_lossy());
            println!("Would you like to create one with defaults (Y/n): ");

            let mut answer = String::new();
            std::io::stdin()
                .read_line(&mut answer)
                .expect("Failed to read user input");

            let config_file = if cfg!(windows) {
                include_str!("../docs/sample_config/default_config_windows.nu")
            } else {
                include_str!("../docs/sample_config/default_config.nu")
            };

            match answer.to_lowercase().trim() {
                "y" | "" => {
                    let mut output = File::create(&config_path).expect("Unable to create file");
                    write!(output, "{}", config_file).expect("Unable to write to config file");
                    println!("Config file created at: {}", config_path.to_string_lossy());
                }
                _ => {
                    println!("Continuing without config file");
                    // Just use the contents of "default_config.nu" or "default_env.nu"
                    eval_source(
                        engine_state,
                        stack,
                        config_file.as_bytes(),
                        "default_config.nu",
                        PipelineData::new(Span::new(0, 0)),
                    );
                    return;
                }
            };
        }

        eval_config_contents(config_path, engine_state, stack);
    }
}
