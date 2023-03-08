use crate::features::TestFeature;
use crate::playground::{CBPlayground, PerTestOptions, RetryExpectations};
use crate::tests::query::create_index;
use crate::{ClusterUnderTest, ConfigAware, TestResult};

use std::ops::Add;

use serde_json::Value;
use std::sync::Arc;
use std::time;
use std::time::Instant;

pub async fn test_should_get_advise_with_context(cluster: Arc<ClusterUnderTest>) -> bool {
    let config = cluster.config();
    if !config.supports_feature(TestFeature::QueryIndexAdvise)
        || !config.supports_feature(TestFeature::Collections)
    {
        return true;
    }

    CBPlayground::setup(
        "test_should_get_advise_with_context",
        cluster.config(),
        PerTestOptions::default().set_no_default_collection(true),
        |dirs, sandbox| {
            let scope = config.scope().unwrap();
            let cmd = format!("cb-env scope \"{}\" |", scope.clone());
            sandbox.set_scope(scope.clone());
            let collection = config.collection().unwrap();
            sandbox.set_collection(collection.clone());

            let fields = "`field1`,`field2`";
            create_index(cmd.clone(), fields, collection, dirs.test(), sandbox);

            let mut advice = Value::default();
            let cmd = format!("{} query advise \"SELECT 1=1\" | first | to json", cmd);
            sandbox.retry_until(
                Instant::now().add(time::Duration::from_secs(30)),
                time::Duration::from_millis(200),
                cmd.as_str(),
                dirs.test(),
                RetryExpectations::ExpectOut,
                |json| -> TestResult<bool> {
                    advice = json.clone();
                    Ok(true)
                },
            );
            assert_eq!("SELECT 1=1", advice["query"]);
            assert_eq!("Advise", advice["#operator"]);
            let inner_advice = advice["advice"].clone();
            assert_eq!("IndexAdvice", inner_advice["#operator"]);
            let advise_info = inner_advice["adviseinfo"].clone();
            assert_ne!("", advise_info["recommended_indexes"]);
        },
    );

    false
}

pub async fn test_should_get_advise(cluster: Arc<ClusterUnderTest>) -> bool {
    let config = cluster.config();
    if !config.supports_feature(TestFeature::QueryIndexAdvise) {
        return true;
    }

    CBPlayground::setup(
        "test_should_get_advise",
        config.clone(),
        PerTestOptions::default().set_no_default_collection(true),
        |dirs, sandbox| {
            let keyspace = config.bucket();

            let fields = "`field1`,`field2`";
            create_index("", fields, keyspace, dirs.test(), sandbox);

            let mut advice = Value::default();
            let cmd = "query advise \"SELECT 1=1\" | first | to json";
            sandbox.retry_until(
                Instant::now().add(time::Duration::from_secs(30)),
                time::Duration::from_millis(200),
                cmd,
                dirs.test(),
                RetryExpectations::ExpectOut,
                |json| -> TestResult<bool> {
                    advice = json.clone();
                    Ok(true)
                },
            );
            assert_eq!("SELECT 1=1", advice["query"]);
            assert_eq!("Advise", advice["#operator"]);
            let inner_advice = advice["advice"].clone();
            assert_eq!("IndexAdvice", inner_advice["#operator"]);
            let advise_info = inner_advice["adviseinfo"].clone();
            assert_ne!("", advise_info["recommended_indexes"]);
        },
    );

    false
}
