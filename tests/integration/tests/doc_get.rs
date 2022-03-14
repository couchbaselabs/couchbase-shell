use crate::cbsh;
use crate::util::{playground, TestConfig};
use nu_test_support::pipeline;
use std::sync::Arc;

pub async fn test_get_a_document(config: Arc<TestConfig>) -> bool {
    playground::CBPlayground::setup("get_a_document", config, |dirs, sandbox| {
        sandbox.create_document(&dirs, "get_a_document", r#"{"testkey": "testvalue"}"#);

        let out = cbsh!(cwd: dirs.test(), pipeline(r#"doc get "get_a_document" | get content | first | to json"#));
        let json = sandbox.parse_out_to_json(out.out);

        assert_eq!("", out.err);
        assert_eq!("testvalue", json["testkey"]);
    });

    false
}

pub async fn test_get_a_document_not_found(config: Arc<TestConfig>) -> bool {
    playground::CBPlayground::setup("get_a_document_not_found", config, |dirs, _sandbox| {
        let out = cbsh!(cwd: dirs.test(), pipeline(r#"doc get "get_a_document_not_found" | get error | first"#));

        assert_eq!("", out.err);
        assert!(out.out.contains("key not found"));
    });

    false
}
