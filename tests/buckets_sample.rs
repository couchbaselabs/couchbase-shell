mod common;

use crate::common::playground::{CBPlayground, RetryExpectations};
use crate::common::TestResult;
use nu_test_support::pipeline;
use std::ops::Add;
use std::time::{Duration, Instant};

#[test]
fn import_sample() {
    CBPlayground::setup("import_sample", None, None, |dirs, sandbox| {
        let out = cbsh!(cwd: dirs.test(), pipeline(r#"buckets load-sample travel-sample | first | to json"#));
        let json = sandbox.parse_out_to_json(out.out).unwrap();

        assert_eq!("", out.err);
        assert_eq!(r#""success""#, json["status"].to_string());

        // Wait for the bucket to finish being created
        sandbox.retry_until(
            Instant::now().add(Duration::from_secs(100)),
            Duration::from_millis(200),
            "buckets get travel-sample | first | to json",
            dirs.test(),
            RetryExpectations::AllowAny {
                allow_err: false,
                allow_out: true,
            },
            |_json| -> TestResult<bool> { Ok(true) },
        );

        // Check bucket has been created
        let out = cbsh!(cwd: dirs.test(), pipeline(r#"buckets get travel-sample | to json"#));
        assert_eq!("", out.err);

        // Cleanup created buckets
        let out = cbsh!(cwd: dirs.test(), pipeline(r#"buckets drop travel-sample"#));
        assert_eq!("", out.err);
    });
}

#[test]
fn import_sample_invalid_sample() {
    CBPlayground::setup("import_sample", None, None, |dirs, sandbox| {
        let out = cbsh!(cwd: dirs.test(), pipeline(r#"buckets load-sample not-a-sample | first | to json"#));
        let json = sandbox.parse_out_to_json(out.out).unwrap();

        assert_eq!("", out.err);
        assert!(json["status"].to_string().contains("Invalid sample bucket"));
    });
}

#[test]
// Skipping for Capella as there is a bug where re-loading already loaded bucket returns 201
// TODO - remove ignore once bug is fixed
#[cfg_attr(feature = "capella", ignore)]
fn import_sample_already_loaded() {
    CBPlayground::setup("import_sample", None, None, |dirs, sandbox| {
        let out = cbsh!(cwd: dirs.test(), pipeline(r#"buckets load-sample beer-sample | first | to json"#));
        let json = sandbox.parse_out_to_json(out.out).unwrap();
        assert_eq!("", out.err);
        assert_eq!(r#""success""#, json["status"].to_string());

        // Wait for the bucket to finish being created
        sandbox.retry_until(
            Instant::now().add(Duration::from_secs(120)),
            Duration::from_millis(200),
            "buckets get beer-sample | first | to json",
            dirs.test(),
            RetryExpectations::AllowAny {
                allow_err: false,
                allow_out: true,
            },
            |_json| -> TestResult<bool> { Ok(true) },
        );

        // Check already_loaded error
        let out = cbsh!(cwd: dirs.test(), pipeline(r#"buckets load-sample beer-sample | first | to json"#));
        let json = sandbox.parse_out_to_json(out.out).unwrap();
        assert_eq!("", out.err);
        assert!(json["status"]
            .to_string()
            .contains("Sample bucket already loaded"));

        // Cleanup created buckets
        let out = cbsh!(cwd: dirs.test(), pipeline(r#"buckets drop beer-sample"#));
        assert_eq!("", out.err);
    });
}
