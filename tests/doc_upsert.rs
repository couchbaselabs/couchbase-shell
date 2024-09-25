mod common;

use crate::common::{playground::CBPlayground, support};

#[test]
#[cfg_attr(not(feature = "key_value"), ignore)]
fn upsert_a_document() {
    CBPlayground::setup("upsert_a_document", None, None, |dirs, sandbox| {
        let out = cbsh!(cwd: dirs.test(), support::cb_pipeline(r#"doc upsert test {"test": "test"} | first | to json"#));

        assert_eq!("", out.err);

        let json = sandbox.parse_out_to_json(out.out).unwrap();

        assert_eq!(1, json["success"]);
        assert_eq!(1, json["processed"]);
        assert_eq!(0, json["failed"]);
        assert_eq!("", json["failures"]);
    });
}

#[test]
#[cfg_attr(not(feature = "key_value"), ignore)]
fn upsert_missing_id() {
    CBPlayground::setup("upsert_missing_id", None, None, |dirs, sandbox| {
        let content = r#"{foo: bar, fizz: buzz}"#;
        let out = cbsh!(cwd: dirs.test(), support::cb_pipeline(format!("{} | wrap content | insert not_id 123 | doc upsert | first | to json", content)));
        assert_eq!("", out.err);

        let json = sandbox.parse_out_to_json(out.out).unwrap();
        assert_eq!(0, json["success"]);
        assert_eq!(1, json["processed"]);
        assert_eq!(1, json["failed"]);
        assert_eq!("Missing doc id", json["failures"]);
    });
}
