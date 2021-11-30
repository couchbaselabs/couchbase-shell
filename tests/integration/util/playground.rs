use crate::util::{fs, TestConfig};
use nu_test_support::playground::*;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;

pub struct CBPlayground {
    bucket: String,
    scope: String,
    collection: String,
    dirs: Dirs,
}

impl CBPlayground {
    pub fn setup(topic: &str, config: Arc<TestConfig>, block: impl FnOnce(&mut CBPlayground)) {
        Playground::setup(topic, |dirs, _sandbox| {
            let mut playground = CBPlayground {
                bucket: config.bucket(),
                scope: config.scope(),
                collection: config.collection(),
                dirs: dirs.clone(),
            };
            let mut config_dir = dirs.test.join(".cbsh".to_string());

            if PathBuf::from(&config_dir).exists() {
                std::fs::remove_dir_all(PathBuf::from(&config_dir))
                    .expect("can not remove cbsh directory");
            }

            std::fs::create_dir(PathBuf::from(&config_dir)).expect("can not create cbsh directory");

            let contents = format!(
                "version = 1
[[clusters]]
identifier = \"local\"
hostnames = [\"{}\"]
default-bucket = \"{}\"
default-collection = \"{}\"
default-scope = \"{}\"
username = \"{}\"
password = \"{}\"
tls-enabled = false",
                config.connstr(),
                config.bucket(),
                config.collection(),
                config.scope(),
                config.username(),
                config.password()
            );

            config_dir.push("config");

            std::fs::write(config_dir, contents.as_bytes()).expect("can not create config file");

            block(&mut playground);
        })
    }

    #[allow(dead_code)]
    pub fn create_document(&self, key: &str, content: &str) {
        let mut command = format!("doc upsert {} {}  --bucket {}", key, content, self.bucket);
        if !self.scope.is_empty() {
            command = format!("{} --scope {}", command, self.scope)
        }
        if !self.collection.is_empty() {
            command = format!("{} --collection {}", command, self.collection)
        }
        command = format!("{} | to json", command);

        let out = self.execute_command(command.as_str());

        assert_eq!("", out.err);

        let json = self.parse_out_to_json(out.out);

        assert_eq!(1, json["success"]);
        assert_eq!(1, json["processed"]);
        assert_eq!(0, json["failed"]);
        assert_eq!(serde_json::Value::Array(vec!()), json["failures"]);
    }

    pub fn execute_command(&self, command: &str) -> Outcome {
        let commands = &*format!(
            "
                        cd \"{}\"
                        {}
                        exit",
            fs::in_directory(&self.dirs.test),
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

        let mut paths = self.shell_os_paths();
        paths.insert(0, test_bins);

        let paths_joined = match std::env::join_paths(paths.iter()) {
            Ok(all) => all,
            Err(_) => panic!("Couldn't join paths for PATH var."),
        };

        let mut process = match Command::new(fs::executable_path())
            .arg("--silent")
            .current_dir(&self.dirs.test)
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

        let out = self.read_std(&output.stdout);
        let err = String::from_utf8_lossy(&output.stderr);

        Outcome::new(out, err.into_owned())
    }

    pub fn read_std(&self, std: &[u8]) -> String {
        let out = String::from_utf8_lossy(std);
        let out = out.lines().collect::<Vec<_>>().join("\n");
        let out = out.replace("\r\n", "");
        out.replace("\n", "")
    }

    pub fn parse_out_to_json(&self, out: String) -> serde_json::Value {
        serde_json::from_str(out.as_str()).unwrap()
    }

    fn shell_os_paths(&self) -> Vec<std::path::PathBuf> {
        let mut original_paths = vec![];

        if let Some(paths) = std::env::var_os("PATH") {
            original_paths = std::env::split_paths(&paths).collect::<Vec<_>>();
        }

        original_paths
    }
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
