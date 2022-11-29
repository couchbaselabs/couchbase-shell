use crate::features::TestFeature;
use crate::support::cb_pipeline;
use crate::util::playground;
use crate::{cbsh, new_doc_id, ClusterUnderTest, ConfigAware};
use std::sync::Arc;

pub async fn test_should_remove_a_document(cluster: Arc<ClusterUnderTest>) -> bool {
    if !cluster.config().supports_feature(TestFeature::KeyValue) {
        return true;
    }

    playground::CBPlayground::setup(
        "should_remove_a_document",
        cluster.config(),
        None,
        |dirs, sandbox| {
            let key = new_doc_id();
            sandbox.create_document(&dirs, &key, r#"{"testkey": "testvalue"}"#);

            let out = cbsh!(cwd: dirs.test(), cb_pipeline(format!("doc remove {} | first | to json", &key)));

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

pub async fn test_should_error_on_remove_doc_not_found(cluster: Arc<ClusterUnderTest>) -> bool {
    if !cluster.config().supports_feature(TestFeature::KeyValue) {
        return true;
    }

    playground::CBPlayground::setup(
        "should_error_on_remove_doc_not_found",
        cluster.config(),
        None,
        |dirs, sandbox| {
            let out =
                cbsh!(cwd: dirs.test(), cb_pipeline("doc remove idontexist | first | to json"));
            assert_eq!("", out.err);

            let json = sandbox.parse_out_to_json(out.out).unwrap();

            assert_eq!(0, json["success"]);
            assert_eq!(1, json["processed"]);
            assert_eq!(1, json["failed"]);
            assert_eq!("Key not found", json["failures"]);
        },
    );

    false
}
