mod common;

use crate::common::playground::CBPlayground;
use nu_test_support::pipeline;

#[test]
#[cfg_attr(not(feature = "key_value"), ignore)]
fn upsert_a_document() {
    CBPlayground::setup("upsert_a_document", None, None, |dirs, sandbox| {
        let out = cbsh!(cwd: dirs.test(), pipeline(r#"doc upsert test {"test": "test"} | first | to json"#));

        assert_eq!("", out.err);

        let json = sandbox.parse_out_to_json(out.out).unwrap();

        assert_eq!(1, json["success"]);
        assert_eq!(1, json["processed"]);
        assert_eq!(0, json["failed"]);
        assert_eq!("", json["failures"]);
    });
}
