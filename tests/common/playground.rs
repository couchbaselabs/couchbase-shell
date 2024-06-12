use crate::cbsh;
use crate::common::config::Config;
use crate::common::{utils, TestConfig, TestResult};
use log::debug;
use nu_test_support::pipeline;
use nu_test_support::playground::{Dirs, Playground};
use reqwest::{Client, ClientBuilder};
use serde_json::{Error, Value};
use std::collections::HashMap;
use std::ops::Add;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread::sleep;
use std::time::{Duration, Instant};
use tokio::task;
use tokio_rustls::rustls::crypto::{aws_lc_rs::default_provider, CryptoProvider};
use tokio_rustls::rustls::ClientConfig;
use uuid::Uuid;

pub struct CBPlayground {
    bucket: String,
    scope: Option<String>,
    collection: Option<String>,
}

#[derive(Default)]
pub struct PerTestOptions {
    no_default_collection: bool,
}

#[allow(dead_code)]
impl PerTestOptions {
    pub fn set_no_default_collection(mut self, no_default_collection: bool) -> PerTestOptions {
        self.no_default_collection = no_default_collection;
        self
    }
}

impl CBPlayground {
    pub fn setup(
        topic: &str,
        config: Option<Arc<TestConfig>>,
        opts: impl Into<Option<PerTestOptions>>,
        block: impl FnOnce(Dirs, &mut CBPlayground),
    ) {
        let config = match config {
            Some(c) => c,
            None => utils::test_config(),
        };

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
    [[cluster]]
    identifier = \"local\"
    connstr = \"{}\"
    default-bucket = \"{}\"
    username = \"{}\"
    password = \"{}\"
    tls-enabled = {}
    tls-accept-all-certs = true
    data-timeout = \"{}\"",
                config.connstr(),
                config.bucket(),
                config.username(),
                config.password(),
                config.connstr().starts_with("couchbases://"),
                config.data_timeout(),
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
    pub fn set_scope(&mut self, scope: String) {
        self.scope = Some(scope);
    }

    #[allow(dead_code)]
    pub fn set_collection(&mut self, collection: String) {
        self.collection = Some(collection);
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

    pub fn parse_out_to_json(&self, out: String) -> Result<Value, Error> {
        serde_json::from_str(out.as_str())
    }

    pub fn retry_until<F>(
        &self,
        deadline: Instant,
        interval: Duration,
        cmd: &str,
        cwd: &Path,
        opts: RetryExpectations,
        mut func: F,
    ) where
        F: FnMut(Value) -> TestResult<bool>,
    {
        let cmd = pipeline(cmd);
        loop {
            if Instant::now() > deadline {
                panic!("Test failed to complete in time");
            }

            let out = cbsh!(cwd, cmd);

            match opts {
                RetryExpectations::ExpectOut => {
                    if out.out.is_empty() {
                        println!("Output from command was empty");
                        sleep(interval);
                        continue;
                    }
                }
                // RetryExpectations::ExpectNoOut => {
                //     if out.out.is_empty() {
                //         return;
                //     } else {
                //         println!("Expected no out but was {}", out.out);
                //         sleep(interval);
                //         continue;
                //     }
                // }
                RetryExpectations::AllowAny {
                    allow_err,
                    allow_out,
                } => {
                    if allow_out && allow_err {
                        println!("Any output and err allowed");
                        return;
                    }

                    if !allow_err && out.err != "" {
                        println!(
                            "Received unexpected content on stderr from command: {}",
                            out.err
                        );
                        sleep(interval);
                        continue;
                    }
                }
            }

            let json = match self.parse_out_to_json(out.out.clone()) {
                Ok(j) => j,
                Err(e) => {
                    println!("Failed to parse {}: {}", out.out, e);
                    sleep(interval);
                    continue;
                }
            };

            match func(json) {
                Ok(success) => {
                    if success {
                        return;
                    }
                    println!("Retry func returned fail")
                }
                Err(e) => println!("Retry func returned error: {}", e),
            }

            sleep(interval);
        }
    }

    pub async fn create_test_config(c: Config) -> Arc<TestConfig> {
        let bucket = c.bucket().unwrap();
        let conn_str = c.conn_string().unwrap();
        let username = c.username().unwrap();
        let password = c.password().unwrap();

        let (scope, collection) = if cfg!(feature = "collections") {
            let client = build_client(conn_str.clone()).await;

            let scope = Self::create_scope(
                client.clone(),
                conn_str.clone(),
                bucket.clone(),
                username.clone(),
                password.clone(),
            )
            .await;
            let collection = Self::create_collection(
                client,
                conn_str.clone(),
                bucket.clone(),
                scope.clone(),
                username.clone(),
                password.clone(),
            )
            .await;

            (Some(scope), Some(collection))
        } else {
            (None, None)
        };

        let config = Arc::new(TestConfig {
            connstr: conn_str.clone(),
            bucket: bucket.clone(),
            scope,
            collection,
            username,
            password,
            data_timeout: c.data_timeout(),
        });
        Self::wait_for_scope(config.clone()).await;
        Self::wait_for_collection(config.clone()).await;
        Self::wait_for_kv(config.clone()).await;
        config
    }

    async fn wait_for_scope(config: Arc<TestConfig>) {
        let scope_name = match config.scope() {
            None => {
                return;
            }
            Some(s) => s,
        };
        Self::setup("wait_for_scope", Some(config), None, |dirs, sandbox| {
            let cmd = r#"scopes | get scope | to json"#;
            sandbox.retry_until(
                Instant::now().add(Duration::from_secs(30)),
                Duration::from_millis(200),
                cmd,
                dirs.test(),
                RetryExpectations::ExpectOut,
                |json| -> TestResult<bool> {
                    for scope in json.as_array().unwrap() {
                        if scope.as_str().unwrap() == scope_name {
                            return Ok(true);
                        }
                    }

                    Ok(false)
                },
            );
        });
    }

    async fn wait_for_kv(config: Arc<TestConfig>) {
        Self::setup("wait_for_kv", Some(config), None, |dirs, sandbox| {
            let cmd = r#"doc upsert wait_for_kv {"test": "test"} | first | to json"#;
            sandbox.retry_until(
                Instant::now().add(Duration::from_secs(30)),
                Duration::from_millis(200),
                cmd,
                dirs.test(),
                RetryExpectations::ExpectOut,
                |json| -> TestResult<bool> {
                    let v = json.as_object().unwrap();
                    match v.get("success") {
                        Some(i) => Ok(i.as_i64().unwrap() == 1),
                        None => Ok(false),
                    }
                },
            );
        });
    }

    async fn wait_for_collection(config: Arc<TestConfig>) {
        let scope_name = match config.scope() {
            None => {
                return;
            }
            Some(s) => s,
        };
        let collection_name = match config.collection() {
            None => {
                return;
            }
            Some(c) => c,
        };
        Self::setup("wait_for_scope", Some(config), None, |dirs, sandbox| {
            let cmd = r#"collections | select scope collection | to json"#;
            sandbox.retry_until(
                Instant::now().add(Duration::from_secs(30)),
                Duration::from_millis(200),
                cmd,
                dirs.test(),
                RetryExpectations::ExpectOut,
                |json| -> TestResult<bool> {
                    for item in json.as_array().unwrap() {
                        if item["scope"] == scope_name {
                            if item["collection"] == collection_name {
                                return Ok(true);
                            }
                        }
                    }

                    Ok(false)
                },
            );
        });
    }

    pub async fn create_scope(
        client: Client,
        conn_string: String,
        bucket: String,
        username: String,
        password: String,
    ) -> String {
        let mut uuid = Uuid::new_v4().to_string();
        uuid.truncate(6);
        let scope_name = format!("test-{}", uuid);

        let mut params = HashMap::new();
        params.insert("name", scope_name.clone());

        let uri = build_uri(conn_string.clone(), bucket, None).await;

        let res = client
            .post(uri)
            .form(&params)
            .basic_auth(username, Some(password))
            .send()
            .await
            .unwrap();

        if !res.status().is_success() {
            panic!("Create scope failed: {}", res.status())
        };

        scope_name
    }

    pub async fn create_collection(
        client: Client,
        conn_string: String,
        bucket: String,
        scope: String,
        username: String,
        password: String,
    ) -> String {
        let mut uuid = Uuid::new_v4().to_string();
        uuid.truncate(6);
        let collection_name = format!("test-{}", uuid);

        let mut params = HashMap::new();
        params.insert("name", collection_name.clone());

        let uri = build_uri(conn_string.clone(), bucket, scope).await;

        let res = client
            .post(uri)
            .form(&params)
            .basic_auth(username, Some(password))
            .send()
            .await
            .unwrap();

        if !res.status().is_success() {
            panic!("Create collection failed: {}", res.status())
        };

        collection_name
    }
}

#[derive(Ord, PartialOrd, Eq, PartialEq)]
pub enum RetryExpectations {
    ExpectOut,
    //ExpectNoOut
    AllowAny { allow_out: bool, allow_err: bool },
}

async fn build_client(conn_string: String) -> Client {
    let mut client_builder = ClientBuilder::new();
    let _ = CryptoProvider::install_default(default_provider());
    let builder = ClientConfig::builder();

    if conn_string.starts_with("couchbases://") {
        let config = builder
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(utilities::InsecureCertVerifier {}))
            .with_no_client_auth();
        client_builder = client_builder.use_preconfigured_tls(config);
    };
    client_builder.build().unwrap()
}

async fn build_uri(
    conn_string: String,
    bucket: String,
    scope: impl Into<Option<String>>,
) -> String {
    if conn_string.starts_with("couchbases://") {
        let seeds = task::spawn_blocking(move || {
            utilities::try_lookup_srv(
                conn_string
                    .clone()
                    .strip_prefix("couchbases://")
                    .unwrap()
                    .to_string(),
            )
            .unwrap()
        })
        .await
        .unwrap();

        if let Some(scope) = scope.into() {
            format!(
                "https://{}:18091/pools/default/buckets/{}/scopes/{}/collections",
                seeds[0], bucket, scope
            )
        } else {
            format!(
                "https://{}:18091/pools/default/buckets/{}/scopes",
                seeds[0], bucket
            )
        }
    } else {
        if let Some(scope) = scope.into() {
            format!(
                "{}/pools/default/buckets/{}/scopes/{}/collections",
                conn_string, bucket, scope
            )
        } else {
            format!("{}/pools/default/buckets/{}/scopes", conn_string, bucket)
        }
    }
}
