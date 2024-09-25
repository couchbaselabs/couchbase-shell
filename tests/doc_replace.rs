mod common;

use crate::common::{new_doc_id, playground::CBPlayground, support};

#[test]
#[cfg_attr(not(feature = "key_value"), ignore)]
fn replace_a_document() {
    CBPlayground::setup("replace_a_document", None, None, |dirs, sandbox| {
        let key = new_doc_id();
        let content = r#"{foo: bar, fizz: buzz}"#;
        let out = cbsh!(cwd: dirs.test(), support::cb_pipeline(format!("{} | wrap content | insert id {} | doc insert | first | to json", content, key)));
        assert_eq!("", out.err);
        let json = sandbox.parse_out_to_json(out.out).unwrap();
        assert_eq!(1, json["success"]);
        assert_eq!(1, json["processed"]);
        assert_eq!(0, json["failed"]);
        assert_eq!("", json["failures"]);

        let replacement_content = r#"{foo: fizz, bar: buzz}"#;
        let out = cbsh!(cwd: dirs.test(), support::cb_pipeline(format!("{} | wrap content | insert id {} | doc replace | first | to json", replacement_content, key)));
        assert_eq!("", out.err);
        let json = sandbox.parse_out_to_json(out.out).unwrap();
        assert_eq!(1, json["success"]);
        assert_eq!(1, json["processed"]);
        assert_eq!(0, json["failed"]);
        assert_eq!("", json["failures"]);

        let out = cbsh!(cwd: dirs.test(), support::cb_pipeline(format!("doc get {} | first | to json", key)));
        assert_eq!("", out.err);
        let json = sandbox.parse_out_to_json(out.out).unwrap();
        assert_eq!(
            "{\"foo\":\"fizz\",\"bar\":\"buzz\"}",
            json["content"].to_string()
        );
    });
}

#[test]
#[cfg_attr(not(feature = "key_value"), ignore)]
fn replace_missing_doc() {
    CBPlayground::setup("replace_missing_doc", None, None, |dirs, sandbox| {
        let key = new_doc_id();
        let content = r#"{foo: bar, fizz: buzz}"#;
        let out = cbsh!(cwd: dirs.test(), support::cb_pipeline(format!("{} | wrap content | insert id {} | doc replace | first | to json", content, key)));
        assert_eq!("", out.err);
        let json = sandbox.parse_out_to_json(out.out).unwrap();

        assert_eq!(0, json["success"]);
        assert_eq!(1, json["processed"]);
        assert_eq!(1, json["failed"]);
        assert_eq!("Key not found", json["failures"]);
    });
}

#[test]
#[cfg_attr(not(feature = "key_value"), ignore)]
fn replace_missing_doc_id() {
    CBPlayground::setup("replace_missing_doc_id", None, None, |dirs, sandbox| {
        let key = new_doc_id();
        let content = r#"{foo: bar, fizz: buzz}"#;
        let out = cbsh!(cwd: dirs.test(), support::cb_pipeline(format!("{} | wrap content | insert not_id {} | doc replace | first | to json", content, key)));
        assert_eq!("", out.err);
        let json = sandbox.parse_out_to_json(out.out).unwrap();

        assert_eq!(0, json["success"]);
        assert_eq!(1, json["processed"]);
        assert_eq!(1, json["failed"]);
        assert_eq!("Missing doc id", json["failures"]);
    });
}
