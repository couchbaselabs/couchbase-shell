use crate::features::TestFeature;
use crate::playground::{CBPlayground, PerTestOptions, RetryExpectations};
use crate::{ClusterUnderTest, ConfigAware, TestResult};

use serde_json::Value;
use std::ops::Add;
use std::path::PathBuf;
use std::sync::Arc;
use std::time;
use std::time::Instant;
use uuid::Uuid;

pub fn create_primary_index(
    base_cmd: impl Into<String>,
    keyspace: String,
    cwd: &PathBuf,
    sandbox: &mut CBPlayground,
) -> TestResult<()> {
    let cmd = format!(
        "{} query \"CREATE PRIMARY INDEX ON `{}`\"",
        base_cmd.into(),
        keyspace
    );
    sandbox.retry_until(
        Instant::now().add(time::Duration::from_secs(30)),
        time::Duration::from_millis(200),
        cmd.as_str(),
        cwd,
        RetryExpectations::AllowAny {
            allow_err: true,
            allow_out: true,
        },
        |_json| -> TestResult<bool> { Ok(true) },
    );
    Ok(())
}

pub fn create_index(
    base_cmd: impl Into<String>,
    fields: impl Into<String>,
    keyspace: String,
    cwd: &PathBuf,
    sandbox: &mut CBPlayground,
) -> String {
    let mut uuid = Uuid::new_v4().to_string();
    uuid.truncate(6);
    let index_name = format!("test-{}", uuid);
    let cmd = format!(
        "{} query \"CREATE INDEX `{}` ON `{}`({})\"",
        base_cmd.into(),
        index_name.clone(),
        keyspace,
        fields.into()
    );
    sandbox.retry_until(
        Instant::now().add(time::Duration::from_secs(30)),
        time::Duration::from_millis(200),
        cmd.as_str(),
        cwd,
        RetryExpectations::AllowAny {
            allow_err: true,
            allow_out: true,
        },
        |_json| -> TestResult<bool> { Ok(true) },
    );

    index_name
}

pub async fn test_should_send_context_with_a_query(cluster: Arc<ClusterUnderTest>) -> bool {
    let config = cluster.config();
    if !config.supports_feature(TestFeature::Query)
        || !config.supports_feature(TestFeature::Collections)
    {
        return true;
    }

    CBPlayground::setup(
        "test_should_send_context_with_a_query",
        cluster.config(),
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
                Instant::now().add(time::Duration::from_secs(30)),
                time::Duration::from_millis(200),
                cmd.as_str(),
                dirs.test(),
                RetryExpectations::ExpectOut,
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

    false
}

pub async fn test_should_execute_a_query(cluster: Arc<ClusterUnderTest>) -> bool {
    let config = cluster.config();
    if !config.supports_feature(TestFeature::Query) {
        return true;
    }

    CBPlayground::setup(
        "test_should_execute_a_query",
        cluster.config(),
        PerTestOptions::default().set_no_default_collection(true),
        |dirs, sandbox| {
            create_primary_index("", config.bucket(), dirs.test(), sandbox).unwrap();
            let key = format!("test-{}", Uuid::new_v4().to_string());
            sandbox.create_document(&dirs, key.clone(), r#"{"testkey": "testvalue"}"#);

            let cmd = format!("query \"SELECT `{0}`.* FROM `{0}` WHERE meta().id=\"{1}\"\" | select testkey | first | to json", config.bucket(), key);
            sandbox.retry_until(
                Instant::now().add(time::Duration::from_secs(30)),
                time::Duration::from_millis(200),
                cmd.as_str(),
                dirs.test(),
                RetryExpectations::ExpectOut,
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

    false
}

pub async fn test_should_fetch_meta(cluster: Arc<ClusterUnderTest>) -> bool {
    let config = cluster.config();
    if !config.supports_feature(TestFeature::Query) {
        return true;
    }

    CBPlayground::setup(
        "test_should_fetch_meta",
        cluster.config(),
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
                Instant::now().add(time::Duration::from_secs(30)),
                time::Duration::from_millis(200),
                cmd.as_str(),
                dirs.test(),
                RetryExpectations::ExpectOut,
                |json| -> TestResult<bool> {
                    val = json.clone();
                    Ok(true)
                },
            );
            assert_eq!(1, val["results"].as_array().unwrap().len());
            assert_ne!("", val["requestID"]);
            assert_eq!("success", val["status"]);
            let metrics = val["metrics"].clone();
            assert_ne!("", metrics["elapsedTime"]);
            assert_ne!("", metrics["executionTime"]);
            assert_eq!(1, metrics["resultCount"]);
            assert_ne!(0, metrics["resultSize"]);
        },
    );

    false
}
