use crate::features::TestFeature;
use crate::support::cb_pipeline;
use crate::util::playground;
use crate::{cbsh, new_doc_id, ClusterUnderTest, ConfigAware};
use std::sync::Arc;

pub async fn test_should_insert_a_document(cluster: Arc<ClusterUnderTest>) -> bool {
    if !cluster.config().supports_feature(TestFeature::KeyValue) {
        return true;
    }

    playground::CBPlayground::setup(
        "should_insert_a_document",
        cluster.config(),
        None,
        |dirs, sandbox| {
            let key = new_doc_id();
            let content = r#"{"test": "test"}"#;
            let out = cbsh!(cwd: dirs.test(), cb_pipeline(format!("doc insert {} {} | first | to json", &key, content)));

            assert_eq!("", out.err);

            let json = sandbox.parse_out_to_json(out.out).unwrap();

            assert_eq!(1, json["success"]);
            assert_eq!(1, json["processed"]);
            assert_eq!(0, json["failed"]);
            assert_eq!("", json["failures"]);
        },
    );

    false
}

pub async fn test_should_error_on_insert_twice(cluster: Arc<ClusterUnderTest>) -> bool {
    if !cluster.config().supports_feature(TestFeature::KeyValue) {
        return true;
    }

    playground::CBPlayground::setup(
        "should_error_on_insert_twice",
        cluster.config(),
        None,
        |dirs, sandbox| {
            let key = new_doc_id();
            let content = r#"{"test": "test"}"#;
            sandbox.create_document(&dirs, key.clone(), r#"{"testkey": "testvalue"}"#);

            let out = cbsh!(cwd: dirs.test(), cb_pipeline(format!("doc insert {} {} | first | to json", &key, content)));

            let json = sandbox.parse_out_to_json(out.out).unwrap();

            assert_eq!(0, json["success"]);
            assert_eq!(1, json["processed"]);
            assert_eq!(1, json["failed"]);
            assert_eq!("Key already exists", json["failures"]);
        },
    );

    false
}
