mod common;

use crate::common::{new_doc_id, playground::CBPlayground, support};

#[test]
#[cfg_attr(not(feature = "key_value"), ignore)]
fn insert_a_document() {
    CBPlayground::setup("insert_a_document", None, None, |dirs, sandbox| {
        let key = new_doc_id();
        let content = r#"{"test": "test"}"#;
        let out = cbsh!(cwd: dirs.test(), support::cb_pipeline(format!("doc insert {} {} | first | to json", &key, content)));

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
fn error_on_insert_twice() {
    CBPlayground::setup("error_on_insert_twice", None, None, |dirs, sandbox| {
        let key = new_doc_id();
        let content = r#"{"test": "test"}"#;
        sandbox.create_document(&dirs, key.clone(), r#"{"testkey": "testvalue"}"#);

        let out = cbsh!(cwd: dirs.test(), support::cb_pipeline(format!("doc insert {} {} | first | to json", &key, content)));

        let json = sandbox.parse_out_to_json(out.out).unwrap();

        assert_eq!(0, json["success"]);
        assert_eq!(1, json["processed"]);
        assert_eq!(1, json["failed"]);
        assert_eq!("Key already exists", json["failures"]);
    });
}
