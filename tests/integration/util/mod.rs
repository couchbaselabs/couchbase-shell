pub mod config;
pub mod features;
pub mod mock;
mod node_version;
pub mod playground;
pub mod standalone;

use crate::util::features::TestFeature;
use crate::util::mock::MockCluster;
use crate::util::standalone::StandaloneCluster;
use std::sync::Arc;

#[derive(Debug)]
pub struct TestConfig {
    connstr: String,
    bucket: String,
    scope: String,
    collection: String,
    username: String,
    password: String,
    support_matrix: Vec<TestFeature>,
    enabled_tests: Vec<String>,
}

impl TestConfig {
    pub fn connstr(&self) -> String {
        self.connstr.clone()
    }
    pub fn bucket(&self) -> String {
        self.bucket.clone()
    }
    pub fn scope(&self) -> String {
        self.scope.clone()
    }
    pub fn collection(&self) -> String {
        self.collection.clone()
    }
    pub fn username(&self) -> String {
        self.username.clone()
    }
    pub fn password(&self) -> String {
        self.password.clone()
    }
    pub fn supports_feature(&self, feature: TestFeature) -> bool {
        self.support_matrix.contains(&feature)
    }
    pub fn supports_features(&self, features: Vec<TestFeature>) -> bool {
        for feature in features {
            if !self.support_matrix.contains(&feature) {
                return false;
            }
        }

        true
    }
    pub fn test_enabled(&self, test: String) -> bool {
        if self.enabled_tests.is_empty() {
            return true;
        }

        self.enabled_tests.contains(&test)
    }
}

pub enum ClusterUnderTest {
    Standalone(StandaloneCluster),
    Mocked(MockCluster),
}

impl ConfigAware for ClusterUnderTest {
    fn config(&self) -> Arc<TestConfig> {
        match self {
            ClusterUnderTest::Standalone(s) => s.config(),
            ClusterUnderTest::Mocked(m) => m.config(),
        }
    }
}

pub trait ConfigAware {
    fn config(&self) -> Arc<TestConfig>;
}
