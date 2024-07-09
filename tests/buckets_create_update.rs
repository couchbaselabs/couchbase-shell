mod common;

use crate::common::playground::CBPlayground;
use nu_test_support::pipeline;
use std::{thread, time};

#[test]
fn create_and_update_bucket() {
    CBPlayground::setup("create_bucket", None, None, |dirs, sandbox| {
        let out = cbsh!(cwd: dirs.test(), pipeline(r#"buckets create test-create-update 256 --type ephemeral --replicas 2 --flush --expiry 100 --durability majority"#));
        assert_eq!("", out.err);

        // Wait for the bucket to finish being created
        thread::sleep(time::Duration::from_millis(5000));

        // Check bucket has been created with correct settings
        let out = cbsh!(cwd: dirs.test(), pipeline(r#"buckets get test-create-update | first | to json"#));
        assert_eq!("", out.err);
        let json = sandbox.parse_out_to_json(out.out).unwrap();

        assert_eq!(json["ram_quota"], 256 * 1024 * 1024);
        assert_eq!(json["type"], "ephemeral");
        assert_eq!(json["replicas"], 2);
        assert_eq!(json["min_durability_level"], "majority");
        assert_eq!(json["flush_enabled"], true);
        assert_eq!(json["max_expiry"], 100);

        let out = cbsh!(cwd: dirs.test(), pipeline(r#"buckets update test-create-update --ram 300 --durability none --replicas 1 --flush false --expiry 90"#));
        assert_eq!("", out.err);

        // Check bucket has been updated with correct settings
        let out = cbsh!(cwd: dirs.test(), pipeline(r#"buckets get test-create-update | first | to json"#));
        let json = sandbox.parse_out_to_json(out.out).unwrap();
        assert_eq!("", out.err);

        assert_eq!(json["ram_quota"], 300 * 1024 * 1024);
        assert_eq!(json["replicas"], 1);
        assert_eq!(json["min_durability_level"], "none");
        assert_eq!(json["flush_enabled"], false);
        assert_eq!(json["max_expiry"], 90);

        let out = cbsh!(cwd: dirs.test(), pipeline(r#"buckets drop test-create-update"#));
        assert_eq!("", out.err);
    });
}
