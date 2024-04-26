pub mod config;
pub mod error;
pub mod playground;
pub mod support;
pub mod utils;

use crate::common::error::TestError;
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
    pub fn data_timeout(&self) -> String {
        self.data_timeout.clone()
    }
}

#[allow(dead_code)]
pub fn new_doc_id() -> String {
    format!("test-{}", Uuid::new_v4().to_string())
}
