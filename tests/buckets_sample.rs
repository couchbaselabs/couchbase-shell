mod common;

use crate::common::playground::CBPlayground;
use nu_test_support::pipeline;
use std::{thread, time};

#[test]
fn import_sample() {
    CBPlayground::setup("import_sample", None, None, |dirs, sandbox| {
        let out = cbsh!(cwd: dirs.test(), pipeline(r#"buckets load-sample travel-sample | first | to json"#));
        let json = sandbox.parse_out_to_json(out.out).unwrap();

        assert_eq!("", out.err);
        assert_eq!(r#""success""#, json["status"].to_string());

        // Wait for the bucket to finish being created
        thread::sleep(time::Duration::from_millis(5000));

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
        assert!(json["status"].to_string().contains("failure - Sample"));
        assert!(json["status"].to_string().contains("not a valid sample"));
    });
}

#[test]
// Skipping for Capella as there is a bug where re-loading already loaded bucket returns 201
// TODO - remove ignore once bug is fixed
#[cfg_attr(feature = "capella", ignore)]
fn import_sample_already_loaded() {
    CBPlayground::setup("import_sample", None, None, |dirs, sandbox| {
        let out = cbsh!(cwd: dirs.test(), pipeline(r#"buckets load-sample beer-sample | first | to json"#));
        assert_eq!("", out.err);

        // Wait for the bucket to finish being created
        thread::sleep(time::Duration::from_secs(5));

        // Check already_loaded error
        // Commented out for now, since v4 returns 201 on loading of already loaded bucket
        let out = cbsh!(cwd: dirs.test(), pipeline(r#"buckets load-sample beer-sample | first | to json"#));
        let json = sandbox.parse_out_to_json(out.out).unwrap();
        assert_eq!("", out.err);
        assert!(json["status"].to_string().contains("failure - Sample"));
        assert!(json["status"].to_string().contains("already loaded"));

        // Cleanup created buckets
        let out = cbsh!(cwd: dirs.test(), pipeline(r#"buckets drop beer-sample"#));
        assert_eq!("", out.err);
    });
}
