pub mod config;
pub mod error;
pub mod features;
pub mod mock;
pub mod playground;
pub mod standalone;

use crate::error::TestError;
use crate::util::features::TestFeature;
use crate::util::mock::MockCluster;
use crate::util::standalone::StandaloneCluster;
use std::sync::Arc;
use uuid::Uuid;

pub type TestResult<T> = Result<T, TestError>;

#[derive(Debug)]
pub struct TestConfig {
    connstr: String,
    bucket: String,
    scope: Option<String>,
    collection: Option<String>,
    username: String,
    password: String,
    support_matrix: Vec<TestFeature>,
    data_timeout: String,
}

impl TestConfig {
    pub fn connstr(&self) -> String {
        self.connstr.clone()
    }
    pub fn bucket(&self) -> String {
        self.bucket.clone()
    }
    pub fn scope(&self) -> Option<String> {
        self.scope.clone()
    }
    pub fn collection(&self) -> Option<String> {
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
    pub fn data_timeout(&self) -> String {
        self.data_timeout.clone()
    }
}

#[derive(Debug)]
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

pub fn new_doc_id() -> String {
    format!("test-{}", Uuid::new_v4().to_string())
}
