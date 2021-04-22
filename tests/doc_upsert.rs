mod common;
//use common::playground;

/*#[test]
pub fn upserts_a_document() {
    playground::CBPlayground::setup("upsert_a_document", |dirs, _sandbox| {
        let out =
            common::execute_command(&dirs.test, r#"doc upsert test {"test": "test"} | to json"#);

        // assert_eq!("", out.err); If we do this then Windows will ALWAYS fail due to the openssl warning

        let json = common::parse_out_to_json(out.out);

        assert_eq!(1, json["success"]);
        assert_eq!(1, json["processed"]);
        assert_eq!(0, json["failed"]);
        assert_eq!(serde_json::Value::Array(vec!()), json["failures"]);
    });
}
*/
