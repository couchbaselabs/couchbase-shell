use super::{ConfigAware, TestConfig};
use crate::util::features::TestFeature;
use crate::{playground, Config, TestResult};
use lazy_static::lazy_static;

use crate::playground::RetryExpectations;
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
        TestFeature::QueryIndexAdvise,
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

        if !c.capella_conn_string().is_none() {
            c.capella_username().unwrap();
            c.capella_password().unwrap();
            c.capella_access_key().unwrap();
            c.capella_secret_key().unwrap();
        }

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
            data_timeout: c.data_timeout(),
            capella_connstr: c.capella_conn_string(),
            capella_username: c.capella_username(),
            capella_password: c.capella_password(),
            capella_access_key: c.capella_access_key(),
            capella_secret_key: c.capella_secret_key(),
        });
        StandaloneCluster::wait_for_scope(config.clone()).await;
        StandaloneCluster::wait_for_collection(config.clone()).await;
        StandaloneCluster::wait_for_kv(config.clone()).await;

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
            let cmd = r#"scopes | get scope | to json"#;
            sandbox.retry_until(
                Instant::now().add(time::Duration::from_secs(30)),
                time::Duration::from_millis(200),
                cmd,
                dirs.test(),
                RetryExpectations::ExpectOut,
                |json| -> TestResult<bool> {
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

    async fn wait_for_kv(config: Arc<TestConfig>) {
        playground::CBPlayground::setup("wait_for_kv", config, None, |dirs, sandbox| {
            let cmd = r#"doc upsert wait_for_kv {"test": "test"} | first | to json"#;
            sandbox.retry_until(
                Instant::now().add(time::Duration::from_secs(30)),
                time::Duration::from_millis(200),
                cmd,
                dirs.test(),
                RetryExpectations::ExpectOut,
                |json| -> TestResult<bool> {
                    let v = json.as_object().unwrap();
                    match v.get("success") {
                        Some(i) => Ok(i.as_i64().unwrap() == 1),
                        None => Ok(false),
                    }
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
            let cmd = r#"collections | select scope collection | to json"#;
            sandbox.retry_until(
                Instant::now().add(time::Duration::from_secs(30)),
                time::Duration::from_millis(200),
                cmd,
                dirs.test(),
                RetryExpectations::ExpectOut,
                |json| -> TestResult<bool> {
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
