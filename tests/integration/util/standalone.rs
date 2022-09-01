use super::{ConfigAware, TestConfig};
use crate::util::features::TestFeature;
use crate::{cbsh, playground, Config, TestResult};
use lazy_static::lazy_static;
use nu_test_support::pipeline;
use serde_json::Value;
use std::collections::HashMap;
use std::ops::Add;
use std::sync::Arc;
use std::time;
use std::time::Instant;
use uuid::Uuid;

lazy_static! {
    static ref ALWAYS_SUPPORTS: Vec<TestFeature> = vec![
        TestFeature::KeyValue,
        TestFeature::Query,
        TestFeature::QueryIndex,
        TestFeature::QueryIndexDefinitions,
    ];
}

#[derive(Debug)]
pub struct StandaloneCluster {
    config: Arc<TestConfig>,
}

impl StandaloneCluster {
    pub async fn start(c: Config) -> Self {
        let bucket = c.bucket().unwrap();
        let conn_str = c.conn_string().unwrap();
        let username = c.username().unwrap();
        let password = c.password().unwrap();
        let features = StandaloneCluster::get_supported_features(
            conn_str.clone(),
            bucket.clone(),
            username.clone(),
            password.clone(),
        )
        .await;
        let (scope, collection) = if features.contains(&TestFeature::Collections) {
            let scope = StandaloneCluster::create_scope(
                conn_str.clone(),
                bucket.clone(),
                username.clone(),
                password.clone(),
            )
            .await;
            let collection = StandaloneCluster::create_collection(
                conn_str.clone(),
                bucket.clone(),
                scope.clone(),
                username.clone(),
                password.clone(),
            )
            .await;

            (Some(scope), Some(collection))
        } else {
            (None, None)
        };

        let enabled_features = if c.enabled_features().is_empty() {
            features
        } else {
            let mut enabled = vec![];
            let config_enabled = c.enabled_features();
            for feature in features {
                if config_enabled.contains(&feature) {
                    enabled.push(feature)
                }
            }
            enabled
        };

        let config = Arc::new(TestConfig {
            connstr: conn_str.clone(),
            bucket: bucket.clone(),
            scope,
            collection,
            username,
            password,
            support_matrix: enabled_features,
        });
        StandaloneCluster::wait_for_scope(config.clone()).await;
        StandaloneCluster::wait_for_collection(config.clone()).await;

        Self { config }
    }

    async fn wait_for_scope(config: Arc<TestConfig>) {
        let scope_name = match config.scope() {
            None => {
                return;
            }
            Some(s) => s,
        };
        playground::CBPlayground::setup("wait_for_scope", config, None, |dirs, sandbox| {
            playground::CBPlayground::retry_until(
                Instant::now().add(time::Duration::from_secs(30)),
                time::Duration::from_millis(200),
                || -> TestResult<bool> {
                    let out = cbsh!(cwd: dirs.test(), pipeline(r#"scopes | get scope | to json"#));

                    assert_eq!("", out.err);

                    let json = sandbox.parse_out_to_json(out.out).unwrap();

                    for scope in json.as_array().unwrap() {
                        if scope.as_str().unwrap() == scope_name {
                            return Ok(true);
                        }
                    }

                    Ok(false)
                },
            );
        });
    }

    async fn wait_for_collection(config: Arc<TestConfig>) {
        let scope_name = match config.scope() {
            None => {
                return;
            }
            Some(s) => s,
        };
        let collection_name = match config.collection() {
            None => {
                return;
            }
            Some(c) => c,
        };
        playground::CBPlayground::setup("wait_for_scope", config, None, |dirs, sandbox| {
            playground::CBPlayground::retry_until(
                Instant::now().add(time::Duration::from_secs(30)),
                time::Duration::from_millis(200),
                || -> TestResult<bool> {
                    let out = cbsh!(cwd: dirs.test(), pipeline(r#"collections | select scope collection | to json"#));

                    assert_eq!("", out.err);

                    let json = sandbox.parse_out_to_json(out.out).unwrap();

                    for item in json.as_array().unwrap() {
                        if item["scope"] == scope_name {
                            if item["collection"] == collection_name {
                                return Ok(true);
                            }
                        }
                    }

                    Ok(false)
                },
            );
        });
    }

    async fn get_supported_features(
        conn_string: String,
        bucket: String,
        username: String,
        password: String,
    ) -> Vec<TestFeature> {
        let uri = format!("{}/pools/default/b/{}", conn_string, bucket);
        let client = reqwest::Client::new();
        let res = client
            .get(uri)
            .basic_auth(username, Some(password))
            .send()
            .await
            .unwrap();
        if !res.status().is_success() {
            panic!("Get bucket config failed: {}", res.status())
        }

        let content: Value = serde_json::from_str(res.text().await.unwrap().as_str()).unwrap();
        let caps = content
            .get("bucketCapabilities")
            .expect("bucketCapabilities not present in payload from cluster");

        let mut features = ALWAYS_SUPPORTS.to_vec();
        for cap in caps.as_array().expect("bucketCapabilities not an array") {
            let c = cap.as_str().expect("bucket capability not a string");
            if c == "collections" {
                features.push(TestFeature::Collections);
            }
        }
        features
    }

    pub async fn create_scope(
        conn_string: String,
        bucket: String,
        username: String,
        password: String,
    ) -> String {
        let uri = format!("{}/pools/default/buckets/{}/scopes", conn_string, bucket);

        let mut uuid = Uuid::new_v4().to_string();
        uuid.truncate(6);
        let scope_name = format!("test-{}", uuid);

        let mut params = HashMap::new();
        params.insert("name", scope_name.clone());

        let client = reqwest::Client::new();
        let res = client
            .post(uri)
            .form(&params)
            .basic_auth(username, Some(password))
            .send()
            .await
            .unwrap();
        if !res.status().is_success() {
            panic!("Create scope failed: {}", res.status())
        };

        scope_name
    }

    pub async fn create_collection(
        conn_string: String,
        bucket: String,
        scope: String,
        username: String,
        password: String,
    ) -> String {
        let uri = format!(
            "{}/pools/default/buckets/{}/scopes/{}/collections",
            conn_string, bucket, scope
        );

        let mut uuid = Uuid::new_v4().to_string();
        uuid.truncate(6);
        let collection_name = format!("test-{}", uuid);

        let mut params = HashMap::new();
        params.insert("name", collection_name.clone());

        let client = reqwest::Client::new();
        let res = client
            .post(uri)
            .form(&params)
            .basic_auth(username, Some(password))
            .send()
            .await
            .unwrap();
        if !res.status().is_success() {
            panic!("Create collection failed: {}", res.status())
        };

        collection_name
    }
}

impl ConfigAware for StandaloneCluster {
    fn config(&self) -> Arc<TestConfig> {
        self.config.clone()
    }
}

impl Drop for StandaloneCluster {
    fn drop(&mut self) {}
}
