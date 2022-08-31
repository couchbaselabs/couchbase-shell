use crate::features::TestFeature;
use crate::playground::PerTestOptions;
use crate::util::playground;
use crate::{cbsh, ClusterUnderTest, ConfigAware};
use nu_test_support::pipeline;
use serde_json::Value;
use std::ops::Add;
use std::path::PathBuf;
use std::sync::Arc;
use std::time;
use std::time::Instant;
use uuid::Uuid;

fn create_primary_index(base_cmd: impl Into<String>, keyspace: String, cwd: &PathBuf) {
    let out = cbsh!(
        cwd,
        pipeline(
            format!(
                "{} query \"CREATE PRIMARY INDEX IF NOT EXISTS ON `{}`\"",
                base_cmd.into(),
                keyspace
            )
            .as_str()
        )
    );

    assert_eq!("", out.out);
    assert_eq!("", out.err);
}

pub async fn test_should_send_context_with_a_query(cluster: Arc<ClusterUnderTest>) -> bool {
    let config = cluster.config();
    if !config.supports_feature(TestFeature::Query)
        || !config.supports_feature(TestFeature::Collections)
    {
        return true;
    }

    playground::CBPlayground::setup(
        "test_should_send_context_with_a_query",
        cluster.config(),
        PerTestOptions::default().set_no_default_collection(true),
        |dirs, sandbox| {
            let (cmd, keyspace) = if let Some(s) = config.scope() {
                (
                    format!("cb-env scope \"{}\" |", s),
                    config.collection().unwrap(),
                )
            } else {
                ("".to_string(), config.bucket())
            };

            create_primary_index(cmd.clone(), keyspace.clone(), dirs.test());
            let key = format!("test-{}", Uuid::new_v4().to_string());
            sandbox.create_document(&dirs, key.clone(), r#"{"testkey": "testvalue"}"#);

            playground::CBPlayground::retry_until(
                Instant::now().add(time::Duration::from_secs(30)),
                time::Duration::from_millis(200),
                || -> bool {
                    let out = cbsh!(cwd: dirs.test(), pipeline(format!("{} query \"SELECT `{}`.* FROM `{}` WHERE meta().id=\"{}\"\" | select testkey | first | to json", cmd, keyspace.clone(), keyspace, key).as_str()));

                    if out.err != "" {
                        println!("Received error from query: {}", out.err);
                        return false;
                    }

                    let json = sandbox.parse_out_to_json(out.out);

                    match json {
                        Ok(v) => {
                            if "testvalue" != v["testkey"] {
                                println!(
                                    "Values do not match: expected testkey = testvalue, actual - {}", v
                                );
                                return false;
                            }
                            true
                        }
                        Err(e) => {
                            println!("Failed to parse json from query: {}", e);
                            false
                        }
                    }
                },
            );
        },
    );

    false
}

pub async fn test_should_execute_a_query(cluster: Arc<ClusterUnderTest>) -> bool {
    let config = cluster.config();
    if !config.supports_feature(TestFeature::Query) {
        return true;
    }

    playground::CBPlayground::setup(
        "test_should_execute_a_query",
        cluster.config(),
        PerTestOptions::default().set_no_default_collection(true),
        |dirs, sandbox| {
            create_primary_index("", config.bucket(), dirs.test());
            let key = format!("test-{}", Uuid::new_v4().to_string());
            sandbox.create_document(&dirs, key.clone(), r#"{"testkey": "testvalue"}"#);

            playground::CBPlayground::retry_until(
                Instant::now().add(time::Duration::from_secs(30)),
                time::Duration::from_millis(200),
                || -> bool {
                    let out = cbsh!(cwd: dirs.test(), pipeline(format!("query \"SELECT `{0}`.* FROM `{0}` WHERE meta().id=\"{1}\"\" | select testkey | first | to json", config.bucket(), key).as_str()));

                    if out.err != "" {
                        println!("Received error from query: {}", out.err);
                        return false;
                    }

                    let json = sandbox.parse_out_to_json(out.out);

                    match json {
                        Ok(v) => {
                            if "testvalue" != v["testkey"] {
                                println!(
                                    "Values do not match: expected testkey = testvalue, actual - {}", v
                                );
                                return false;
                            }
                            true
                        }
                        Err(e) => {
                            println!("Failed to parse json from query: {}", e);
                            false
                        }
                    }
                },
            );
        },
    );

    false
}

pub async fn test_should_fetch_meta(cluster: Arc<ClusterUnderTest>) -> bool {
    let config = cluster.config();
    if !config.supports_feature(TestFeature::Query) {
        return true;
    }

    playground::CBPlayground::setup(
        "test_should_fetch_meta",
        cluster.config(),
        PerTestOptions::default().set_no_default_collection(true),
        |dirs, sandbox| {
            create_primary_index("", config.bucket(), dirs.test());
            let key = format!("test-{}", Uuid::new_v4().to_string());
            sandbox.create_document(&dirs, key.clone(), r#"{"testkey": "testvalue"}"#);

            let mut val: Value = Value::default();
            playground::CBPlayground::retry_until(
                Instant::now().add(time::Duration::from_secs(30)),
                time::Duration::from_millis(200),
                || -> bool {
                    let out = cbsh!(cwd: dirs.test(), pipeline(format!("query \"SELECT `{0}`.* FROM `{0}` WHERE meta().id=\"{1}\"\" --with-meta | flatten -a | first | to json", config.bucket(), key).as_str()));

                    if out.err != "" {
                        println!("Received error from query: {}", out.err);
                        return false;
                    }

                    let json = sandbox.parse_out_to_json(out.out);

                    match json {
                        Ok(v) => {
                            val = v.clone();
                            true
                        }
                        Err(e) => {
                            println!("Failed to parse json from query: {}", e);
                            false
                        }
                    }
                },
            );
            assert_eq!("testvalue", val["testkey"]);
            assert_ne!("", val["elapsedTime"]);
            assert_ne!("", val["executionTime"]);
            assert_eq!(1, val["resultCount"]);
            assert_ne!(0, val["resultSize"]);
            assert_ne!("", val["requestID"]);
            assert_eq!("success", val["status"]);
        },
    );

    false
}
