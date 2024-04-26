mod common;

use crate::common::{new_doc_id, playground::CBPlayground, support};

#[test]
#[cfg_attr(not(feature = "key_value"), ignore)]
fn remove_a_document() {
    CBPlayground::setup("remove_a_document", None, None, |dirs, sandbox| {
        let key = new_doc_id();
        sandbox.create_document(&dirs, &key, r#"{"testkey": "testvalue"}"#);

        let out = cbsh!(cwd: dirs.test(), support::cb_pipeline(format!("doc remove {} | first | to json", &key)));

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
fn error_on_remove_doc_not_found() {
    CBPlayground::setup(
        "error_on_remove_doc_not_found",
        None,
        None,
        |dirs, sandbox| {
            let out = cbsh!(cwd: dirs.test(), support::cb_pipeline("doc remove idontexist | first | to json"));
            assert_eq!("", out.err);

            let json = sandbox.parse_out_to_json(out.out).unwrap();

            assert_eq!(0, json["success"]);
            assert_eq!(1, json["processed"]);
            assert_eq!(1, json["failed"]);
            assert_eq!("Key not found", json["failures"]);
        },
    );
}
