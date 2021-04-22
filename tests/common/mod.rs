mod fs;
//pub mod playground;

use std::io::prelude::*;
use std::panic;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn shell_os_paths() -> Vec<std::path::PathBuf> {
    let mut original_paths = vec![];

    if let Some(paths) = std::env::var_os("PATH") {
        original_paths = std::env::split_paths(&paths).collect::<Vec<_>>();
    }

    original_paths
}

pub fn execute_command(cwd: &PathBuf, command: &str) -> Outcome {
    let commands = &*format!(
        "
                        cd \"{}\"
                        {}
                        exit",
        fs::in_directory(&cwd),
        fs::DisplayPath::display_path(&command)
    );

    let test_bins = fs::binaries();
    let test_bins = dunce::canonicalize(&test_bins).unwrap_or_else(|e| {
        panic!(
            "Couldn't canonicalize dummy binaries path {}: {:?}",
            test_bins.display(),
            e
        )
    });

    let mut paths = shell_os_paths();
    paths.insert(0, test_bins);

    let paths_joined = match std::env::join_paths(paths.iter()) {
        Ok(all) => all,
        Err(_) => panic!("Couldn't join paths for PATH var."),
    };

    let mut process = match Command::new(fs::executable_path())
        .current_dir(&cwd)
        .env("PATH", paths_joined)
        .stdout(Stdio::piped())
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(why) => panic!("Can't run test {}", why.to_string()),
    };

    let stdin = process.stdin.as_mut().expect("couldn't open stdin");
    stdin
        .write_all(commands.as_bytes())
        .expect("couldn't write to stdin");

    let output = process
        .wait_with_output()
        .expect("couldn't read from stdout/stderr");

    let out = read_std(&output.stdout);
    let err = String::from_utf8_lossy(&output.stderr);

    Outcome::new(out, err.into_owned())
}

pub struct Outcome {
    pub out: String,
    pub err: String,
}

impl Outcome {
    pub fn new(out: String, err: String) -> Outcome {
        Outcome { out, err }
    }
}

pub fn read_std(std: &[u8]) -> String {
    let out = String::from_utf8_lossy(std);
    let out = out.lines().collect::<Vec<_>>().join("\n");
    let out = out.replace("\r\n", "");
    out.replace("\n", "")
}

pub fn parse_out_to_json(out: String) -> serde_json::Value {
    serde_json::from_str(out.as_str()).unwrap()
}
