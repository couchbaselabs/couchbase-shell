mod common;

use crate::common::{playground, playground::PerTestOptions, utils, TestResult};
use serde_json::Value;
use std::ops::Add;
use std::path::Path;
use std::time::{Duration, Instant};
use uuid::Uuid;

pub fn create_primary_index(
    base_cmd: impl Into<String>,
    keyspace: String,
    cwd: &Path,
    sandbox: &mut playground::CBPlayground,
) -> TestResult<()> {
    let cmd = format!(
        "{} query \"CREATE PRIMARY INDEX ON `{}`\"",
        base_cmd.into(),
        keyspace
    );
    sandbox.retry_until(
        Instant::now().add(Duration::from_secs(30)),
        Duration::from_millis(200),
        cmd.as_str(),
        cwd,
        playground::RetryExpectations::AllowAny {
            allow_err: true,
            allow_out: true,
        },
        |_json| -> TestResult<bool> { Ok(true) },
    );
    Ok(())
}

#[test]
#[cfg_attr(not(feature = "query"), ignore)]
#[cfg_attr(not(feature = "collections"), ignore)]
fn send_context_with_a_query() {
    let config = utils::test_config();

    playground::CBPlayground::setup(
        "send_context_with_a_query",
        None,
        PerTestOptions::default().set_no_default_collection(true),
        |dirs, sandbox| {
            let scope = config.scope().unwrap();
            let cmd = format!("cb-env scope \"{}\" |", scope.clone());
            sandbox.set_scope(scope);
            let collection = config.collection().unwrap();
            sandbox.set_collection(collection.clone());

            create_primary_index(cmd.clone(), collection.clone(), dirs.test(), sandbox).unwrap();
            let key = format!("test-{}", Uuid::new_v4().to_string());
            sandbox.create_document(&dirs, key.clone(), r#"{"testkey": "testvalue"}"#);

            let cmd = format!("{0} query \"SELECT `{1}`.* FROM `{1}` WHERE meta().id=\"{2}\"\" | select testkey | first | to json", cmd, collection, key);
            sandbox.retry_until(
                Instant::now().add(Duration::from_secs(30)),
                Duration::from_millis(200),
                cmd.as_str(),
                dirs.test(),
                playground::RetryExpectations::ExpectOut,
                |json| -> TestResult<bool> {
                    if "testvalue" != json["testkey"] {
                        println!(
                            "Values do not match: expected testkey = testvalue, actual - {}",
                            json
                        );
                        return Ok(false);
                    }
                    Ok(true)
                },
            );
        },
    );
}

#[test]
#[cfg_attr(not(feature = "query"), ignore)]
fn execute_a_query() {
    let config = utils::test_config();

    playground::CBPlayground::setup(
        "execute_a_query",
        None,
        PerTestOptions::default().set_no_default_collection(true),
        |dirs, sandbox| {
            create_primary_index("", config.bucket(), dirs.test(), sandbox).unwrap();
            let key = format!("test-{}", Uuid::new_v4().to_string());
            sandbox.create_document(&dirs, key.clone(), r#"{"testkey": "testvalue"}"#);

            let cmd = format!("query \"SELECT `{0}`.* FROM `{0}` WHERE meta().id=\"{1}\"\" | select testkey | first | to json", config.bucket(), key);
            sandbox.retry_until(
                Instant::now().add(Duration::from_secs(30)),
                Duration::from_millis(200),
                cmd.as_str(),
                dirs.test(),
                playground::RetryExpectations::ExpectOut,
                |json| -> TestResult<bool> {
                    if "testvalue" != json["testkey"] {
                        println!(
                            "Values do not match: expected testkey = testvalue, actual - {}",
                            json
                        );
                        return Ok(false);
                    }
                    Ok(true)
                },
            );
        },
    );
}

#[test]
#[cfg_attr(not(feature = "query"), ignore)]
fn fetch_meta() {
    let config = utils::test_config();

    playground::CBPlayground::setup(
        "fetch_meta",
        None,
        PerTestOptions::default().set_no_default_collection(true),
        |dirs, sandbox| {
            create_primary_index("", config.bucket(), dirs.test(), sandbox).unwrap();
            let key = format!("test-{}", Uuid::new_v4().to_string());
            sandbox.create_document(&dirs, key.clone(), r#"{"testkey": "testvalue"}"#);

            let mut val: Value = Value::default();
            let cmd = format!(
                    "query \"SELECT `{0}`.* FROM `{0}` WHERE meta().id=\"{1}\"\" --with-meta | first | to json",
                    config.bucket(),
                    key
                );
            sandbox.retry_until(
                Instant::now().add(Duration::from_secs(30)),
                Duration::from_millis(200),
                cmd.as_str(),
                dirs.test(),
                playground::RetryExpectations::ExpectOut,
                |json| -> TestResult<bool> {
                    val = json.clone();
                    // Wait until the document has been indexed
                    Ok(val["results"].as_array().unwrap().len() == 1)
                },
            );
            assert_ne!("", val["requestID"]);
            assert_eq!("success", val["status"]);
            let metrics = val["metrics"].clone();
            assert_ne!("", metrics["elapsedTime"]);
            assert_ne!("", metrics["executionTime"]);
            assert_eq!(1, metrics["resultCount"]);
            assert_ne!(0, metrics["resultSize"]);
        },
    );
}
