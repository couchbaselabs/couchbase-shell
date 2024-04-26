mod common;

use crate::common::playground::CBPlayground;
use nu_test_support::pipeline;

#[test]
#[cfg_attr(not(feature = "key_value"), ignore)]
fn get_a_document() {
    CBPlayground::setup("get_a_document", None, None, |dirs, sandbox| {
        sandbox.create_document(&dirs, "get_a_document", r#"{"testkey": "testvalue"}"#);

        let out =
            cbsh!(cwd: dirs.test(), pipeline(r#"doc get "get_a_document" | first | to json"#));
        let json = sandbox.parse_out_to_json(out.out).unwrap();

        assert_eq!("", out.err);
        assert_eq!(r#"{"testkey":"testvalue"}"#, json["content"].to_string());
    });
}

#[test]
#[cfg_attr(not(feature = "key_value"), ignore)]
fn get_a_document_not_found() {
    CBPlayground::setup("get_a_document_not_found", None, None, |dirs, _sandbox| {
        let out = cbsh!(cwd: dirs.test(), pipeline(r#"doc get "get_a_document_not_found" | first | to json"#));

        assert_eq!("", out.err);
        assert!(out.out.contains("Key not found"));
    });
}
