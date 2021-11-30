use crate::util::{playground, TestConfig};
use std::sync::Arc;

pub async fn test_get_a_document(config: Arc<TestConfig>) -> bool {
    playground::CBPlayground::setup("get_a_document", config, |sandbox| {
        sandbox.create_document("get_a_document", r#"{"testkey": "testvalue"}"#);

        let out = sandbox.execute_command(r#"doc get "get_a_document" | get content | to json"#);

        let json = sandbox.parse_out_to_json(out.out);

        assert_eq!("", out.err);
        assert_eq!("testvalue", json["testkey"]);
    });

    false
}

pub async fn test_get_a_document_not_found(config: Arc<TestConfig>) -> bool {
    playground::CBPlayground::setup("get_a_document_not_found", config, |sandbox| {
        let out = sandbox.execute_command(r#"doc get "get_a_document_not_found" | get error"#);

        assert_eq!("", out.err);
        assert!(out.out.contains("key not found"));
    });

    false
}
