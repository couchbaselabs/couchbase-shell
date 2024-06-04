mod common;

use crate::common::playground::CBPlayground;
use nu_test_support::pipeline;
use std::{thread, time};

#[test]
fn import_sample() {
    CBPlayground::setup("import_sample", None, None, |dirs, sandbox| {
        let out =
            cbsh!(cwd: dirs.test(), pipeline(r#"buckets load-sample travel-sample | to json"#));
        let json = sandbox.parse_out_to_json(out.out).unwrap();

        assert_eq!("", out.err);
        assert_eq!(r#""success""#, json[0]["status"].to_string());

        // Wait for the bucket to finish being created
        thread::sleep(time::Duration::from_millis(5000));

        // Check already_loaded error
        let out =
            cbsh!(cwd: dirs.test(), pipeline(r#"buckets load-sample travel-sample | to json"#));
        let json = sandbox.parse_out_to_json(out.out).unwrap();
        assert_eq!("", out.err);
        assert!(json[0]["status"].to_string().contains("failure - Sample"));
        assert!(json[0]["status"].to_string().contains("already loaded"));

        //Create a bucket that takes up the remainder of the available RAM
        let out = cbsh!(cwd: dirs.test(), pipeline(r#"buckets create temp 1650 | to json"#));
        assert_eq!("", out.err);

        //Check not enough memeory error
        let out = cbsh!(cwd: dirs.test(), pipeline(r#"buckets load-sample beer-sample | to json"#));
        let json = sandbox.parse_out_to_json(out.out).unwrap();
        assert_eq!("", out.err);
        assert!(json[0]["status"]
            .to_string()
            .contains("failure - Not enough Quota"));

        // Cleanup created buckets
        let out = cbsh!(cwd: dirs.test(), pipeline(r#"buckets drop travel-sample"#));
        assert_eq!("", out.err);

        let out = cbsh!(cwd: dirs.test(), pipeline(r#"buckets drop temp"#));
        assert_eq!("", out.err);
    });
}

#[test]
fn import_sample_invalid_sample() {
    CBPlayground::setup("import_sample", None, None, |dirs, sandbox| {
        let out =
            cbsh!(cwd: dirs.test(), pipeline(r#"buckets load-sample not-a-sample | to json"#));
        let json = sandbox.parse_out_to_json(out.out).unwrap();

        assert_eq!("", out.err);
        assert!(json[0]["status"].to_string().contains("failure - Sample"));
        assert!(json[0]["status"].to_string().contains("not a valid sample"));
    });
}
