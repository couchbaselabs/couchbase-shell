use lazy_static::lazy_static;
use nu_test_support::playground::*;
use std::env;
use std::path::PathBuf;

lazy_static! {
    static ref STATE: State = State {
        hostnames: env::var("CBSH_HOSTNAMES")
            .unwrap_or("localhost".to_string())
            .split(',')
            .map(|v| v.to_owned())
            .collect(),
        bucket: env::var("CBSH_BUCKET").unwrap_or("default".to_string()),
        scope: env::var("CBSH_SCOPE").unwrap_or("".to_string()),
        collection: env::var("CBSH_COLLECTION").unwrap_or("".to_string()),
        username: env::var("CBSH_USERNAME").unwrap_or("Administrator".to_string()),
        password: env::var("CBSH_PASSWORD").unwrap_or("password".to_string()),
    };
}

// dead_code seems to pick up several functions in this file even though they are used.
#[allow(dead_code)]
pub fn default_bucket() -> String {
    STATE.bucket.clone()
}

#[allow(dead_code)]
pub fn default_scope() -> String {
    STATE.scope.clone()
}

#[allow(dead_code)]
pub fn default_collection() -> String {
    STATE.collection.clone()
}

struct State {
    hostnames: Vec<String>,
    bucket: String,
    scope: String,
    collection: String,
    username: String,
    password: String,
}

pub struct CBPlayground {}

impl CBPlayground {
    pub fn setup(topic: &str, block: impl FnOnce(Dirs, &mut CBPlayground)) {
        Playground::setup(topic, |dirs, _sandbox| {
            let mut playground = CBPlayground {};
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
                STATE.hostnames[0],
                STATE.bucket,
                STATE.collection,
                STATE.scope,
                STATE.username,
                STATE.password
            );

            config_dir.push("config");

            std::fs::write(config_dir, contents.as_bytes()).expect("can not create config file");

            block(dirs, &mut playground);
        })
    }
}
