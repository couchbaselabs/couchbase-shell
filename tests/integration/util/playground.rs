use crate::util::TestConfig;
use crate::{cbsh, TestResult};
use log::debug;
use nu_test_support::pipeline;
use nu_test_support::playground::*;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread::sleep;
use std::time::{Duration, Instant};

pub struct CBPlayground {
    bucket: String,
    scope: Option<String>,
    collection: Option<String>,
}

#[derive(Default)]
pub struct PerTestOptions {
    no_default_collection: bool,
}

impl PerTestOptions {
    pub fn set_no_default_collection(mut self, no_default_collection: bool) -> PerTestOptions {
        self.no_default_collection = no_default_collection;
        self
    }
}

impl CBPlayground {
    pub fn setup(
        topic: &str,
        config: Arc<TestConfig>,
        opts: impl Into<Option<PerTestOptions>>,
        block: impl FnOnce(Dirs, &mut CBPlayground),
    ) {
        Playground::setup(topic, |dirs, _sandbox| {
            let add_collection = if let Some(o) = opts.into() {
                !o.no_default_collection
            } else {
                true
            };
            let mut playground = if add_collection {
                CBPlayground {
                    bucket: config.bucket(),
                    scope: config.scope(),
                    collection: config.collection(),
                }
            } else {
                CBPlayground {
                    bucket: config.bucket(),
                    scope: None,
                    collection: None,
                }
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

            if add_collection {
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
            }

            config_dir.push("config");

            std::fs::write(config_dir, contents.as_bytes()).expect("can not create config file");

            block(dirs, &mut playground);
        })
    }

    #[allow(dead_code)]
    pub fn create_document(&self, dirs: &Dirs, key: impl Into<String>, content: impl Into<String>) {
        let doc_key = key.into();
        debug!("Creating doc: {}", &doc_key);
        let mut command = format!(
            "doc upsert {} {}  --bucket {}",
            doc_key.clone(),
            content.into(),
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

        debug!("Created doc: {}", &doc_key);

        assert_eq!("", out.err);

        let json = self.parse_out_to_json(out.out).unwrap();

        let arr = json.as_array().unwrap();
        assert_eq!(1, arr.len());

        let item = arr.get(0).unwrap();

        assert_eq!(1, item["success"]);
        assert_eq!(1, item["processed"]);
        assert_eq!(0, item["failed"]);
        assert_eq!("", item["failures"]);
    }

    pub fn parse_out_to_json(&self, out: String) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::from_str(out.as_str())
    }

    pub fn retry_until<F>(deadline: Instant, interval: Duration, mut func: F)
    where
        F: FnMut() -> TestResult<bool>,
    {
        loop {
            if Instant::now() > deadline {
                panic!("Test failed to complete in time");
            }

            match func() {
                Ok(success) => {
                    if success {
                        return;
                    }
                }
                Err(e) => {
                    println!("Retry func returned error: {}", e)
                }
            }

            sleep(interval);
        }
    }
}
