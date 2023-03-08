use crate::features::TestFeature;
use crate::util::playground;
use crate::{cbsh, ClusterUnderTest, ConfigAware};
use nu_test_support::pipeline;
use std::sync::Arc;

pub async fn test_get_a_document(cluster: Arc<ClusterUnderTest>) -> bool {
    if !cluster.config().supports_feature(TestFeature::KeyValue) {
        return true;
    }

    playground::CBPlayground::setup("get_a_document", cluster.config(), None, |dirs, sandbox| {
        sandbox.create_document(&dirs, "get_a_document", r#"{"testkey": "testvalue"}"#);

        let out =
            cbsh!(cwd: dirs.test(), pipeline(r#"doc get "get_a_document" | first | to json"#));
        let json = sandbox.parse_out_to_json(out.out).unwrap();

        assert_eq!("", out.err);
        assert_eq!(r#"{"testkey":"testvalue"}"#, json["content"].to_string());
    });

    false
}

pub async fn test_get_a_document_not_found(cluster: Arc<ClusterUnderTest>) -> bool {
    if !cluster.config().supports_feature(TestFeature::KeyValue) {
        return true;
    }

    playground::CBPlayground::setup(
        "get_a_document_not_found",
        cluster.config(),
        None,
        |dirs, _sandbox| {
            let out = cbsh!(cwd: dirs.test(), pipeline(r#"doc get "get_a_document_not_found" | first | to json"#));

            assert_eq!("", out.err);
            assert!(out.out.contains("Key not found"));
        },
    );

    false
}
