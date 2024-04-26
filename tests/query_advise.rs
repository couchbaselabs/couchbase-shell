mod common;

use crate::common::{playground, playground::PerTestOptions, utils, TestResult};
use serde_json::Value;
use std::ops::Add;
use std::time::{Duration, Instant};

#[test]
#[cfg_attr(not(feature = "query_index_advise"), ignore)]
#[cfg_attr(not(feature = "collections"), ignore)]
fn get_advise_with_context() {
    let config = utils::test_config();

    playground::CBPlayground::setup(
        "get_advise_with_context",
        Some(config.clone()),
        PerTestOptions::default().set_no_default_collection(true),
        |dirs, sandbox| {
            let scope = config.scope().unwrap();
            let cmd = format!("cb-env scope \"{}\" |", scope.clone());
            sandbox.set_scope(scope.clone());
            let collection = config.collection().unwrap();
            sandbox.set_collection(collection.clone());

            let fields = "`field1`,`field2`";
            utils::create_index(cmd.clone(), fields, collection, dirs.test(), sandbox);

            let mut advice = Value::default();
            let cmd = format!("{} query advise \"SELECT 1=1\" | first | to json", cmd);
            sandbox.retry_until(
                Instant::now().add(Duration::from_secs(30)),
                Duration::from_millis(200),
                cmd.as_str(),
                dirs.test(),
                playground::RetryExpectations::ExpectOut,
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
}
