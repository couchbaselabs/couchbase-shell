use crate::cbsh;

use crate::util::TestConfig;
use log::debug;
use nu_test_support::pipeline;
use nu_test_support::playground::*;
use std::path::PathBuf;

use std::sync::Arc;

pub struct CBPlayground {
    bucket: String,
    scope: Option<String>,
    collection: Option<String>,
}

impl CBPlayground {
    pub fn setup(
        topic: &str,
        config: Arc<TestConfig>,
        block: impl FnOnce(Dirs, &mut CBPlayground),
    ) {
        Playground::setup(topic, |dirs, _sandbox| {
            let mut playground = CBPlayground {
                bucket: config.bucket(),
                scope: config.scope(),
                collection: config.collection(),
            };
            let mut config_dir = dirs.clone().test.join(".cbsh".to_string());

            if PathBuf::from(&config_dir).exists() {
                std::fs::remove_dir_all(PathBuf::from(&config_dir))
                    .expect("can not remove cbsh directory");
            }

            std::fs::create_dir(PathBuf::from(&config_dir)).expect("can not create cbsh directory");

            let mut contents = format!(
                "version = 1
[[clusters]]
identifier = \"local\"
hostnames = [\"{}\"]
default-bucket = \"{}\"
username = \"{}\"
password = \"{}\"
tls-enabled = false",
                config.connstr(),
                config.bucket(),
                config.username(),
                config.password()
            );
            if let Some(s) = config.scope() {
                contents = format!(
                    "
{}
default-scope = \"{}\"
                ",
                    contents, s
                );
            }
            if let Some(c) = config.collection() {
                contents = format!(
                    "
{}
default-collection = \"{}\"
                ",
                    contents, c
                );
            }

            config_dir.push("config");

            std::fs::write(config_dir, contents.as_bytes()).expect("can not create config file");

            block(dirs, &mut playground);
        })
    }

    #[allow(dead_code)]
    pub fn create_document(&self, dirs: &Dirs, key: &str, content: &str) {
        debug!("Creating doc: {}", &key);
        let mut command = format!(
            "doc upsert {} {}  --bucket {}",
            key.clone(),
            content,
            self.bucket
        );
        if let Some(s) = &self.scope {
            command = format!("{} --scope {}", command, s)
        }
        if let Some(c) = &self.collection {
            command = format!("{} --collection {}", command, c)
        }
        command = format!("{} | to json", command);

        let out = cbsh!(cwd: dirs.test(), pipeline(command.as_str()));

        debug!("Created doc: {}", &key);

        assert_eq!("", out.err);

        let json = self.parse_out_to_json(out.out);

        let arr = json.as_array().unwrap();
        assert_eq!(1, arr.len());

        let item = arr.get(0).unwrap();

        assert_eq!(1, item["success"]);
        assert_eq!(1, item["processed"]);
        assert_eq!(0, item["failed"]);
        assert_eq!("", item["failures"]);
    }

    pub fn parse_out_to_json(&self, out: String) -> serde_json::Value {
        serde_json::from_str(out.as_str()).unwrap()
    }
}
