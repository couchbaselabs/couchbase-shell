use crate::util::{playground, TestConfig};
use std::sync::Arc;

pub async fn test_upserts_a_document(config: Arc<TestConfig>) -> bool {
    playground::CBPlayground::setup("upsert_a_document", config, |sandbox| {
        let out = sandbox.execute_command(r#"doc upsert test {"test": "test"} | to json"#);

        assert_eq!("", out.err);

        let json = sandbox.parse_out_to_json(out.out);

        assert_eq!(1, json["success"]);
        assert_eq!(1, json["processed"]);
        assert_eq!(0, json["failed"]);
        assert_eq!(serde_json::Value::Array(vec!()), json["failures"]);
    });

    false
}
