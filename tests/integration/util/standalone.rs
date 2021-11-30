use super::{ConfigAware, TestConfig};
use crate::util::config::StandaloneConfig;
use crate::util::features::TestFeature;
use crate::util::node_version::NodeVersion;
use lazy_static::lazy_static;
use std::sync::Arc;
use strum::IntoEnumIterator;

lazy_static! {
    pub static ref SERVER_VERSION_650: NodeVersion = NodeVersion {
        major: 6,
        minor: 5,
        patch: 0,
    };
    pub static ref SERVER_VERSION_700: NodeVersion = NodeVersion {
        major: 7,
        minor: 0,
        patch: 0,
    };
}

pub struct StandaloneCluster {
    config: Arc<TestConfig>,
}

impl StandaloneCluster {
    fn supports_features(_version: NodeVersion) -> Vec<TestFeature> {
        let mut features = vec![];
        for feature in TestFeature::iter() {
            match feature {
                TestFeature::KeyValue => {
                    features.push(feature.clone());
                }
                TestFeature::Query => {
                    features.push(feature.clone());
                }
            }
        }

        features
    }

    pub fn start(c: StandaloneConfig, tests: Vec<String>) -> Self {
        let version = match c.server_version() {
            Some(v) => NodeVersion::from(v),
            None => SERVER_VERSION_700.clone(),
        };

        Self {
            config: Arc::new(TestConfig {
                connstr: c.conn_string(),
                bucket: c.default_bucket().unwrap_or_else(|| "default".into()),
                scope: c.default_scope().unwrap_or_else(|| "_default".into()),
                collection: c.default_collection().unwrap_or_else(|| "_default".into()),
                username: c.username(),
                password: c.password(),
                support_matrix: StandaloneCluster::supports_features(version),
                enabled_tests: tests,
            }),
        }
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
