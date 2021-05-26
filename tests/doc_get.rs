mod common;
use common::playground;

#[test]
pub fn get_a_document() {
    playground::CBPlayground::setup("get_a_document", |dirs, _sandbox| {
        common::create_document(
            &dirs.test,
            playground::default_bucket(),
            playground::default_scope(),
            playground::default_collection(),
            "get_a_document",
            r#"{"testkey": "testvalue"}"#,
        );

        let out = common::execute_command(
            &dirs.test,
            r#"doc get "get_a_document" | get content | to json"#,
        );

        let json = common::parse_out_to_json(out.out);

        assert_eq!("", out.err);
        assert_eq!("testvalue", json["testkey"]);
    });
}

#[test]
pub fn get_a_document_not_found() {
    playground::CBPlayground::setup("get_a_document_not_found", |dirs, _sandbox| {
        let out = common::execute_command(
            &dirs.test,
            r#"doc get "get_a_document_not_found" | get error"#,
        );

        assert_eq!("", out.err);
        assert!(out.out.contains("key not found"));
    });
}
