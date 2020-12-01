mod common;
use common::playground;
use futures::executor::block_on;
use std::collections::HashMap;

#[test]
pub fn get_a_document() {
    playground::CBPlayground::setup("get_a_document", |dirs, sandbox| {
        let mut content = HashMap::new();
        content.insert("Hello", "Rust!");

        block_on(sandbox.with_document(
            playground::default_bucket(),
            playground::default_scope(),
            playground::default_collection(),
            "get_a_document".into(),
            content,
        ))
        .unwrap();

        let out = common::execute_command(
            &dirs.test,
            r#"doc get "get_a_document" | get content | to json"#,
        );

        let json = common::parse_out_to_json(out.out);

        assert_eq!("", out.err);
        assert_eq!("Rust!", json["Hello"]);
    });
}
