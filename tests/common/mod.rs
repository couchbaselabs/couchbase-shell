pub mod config;
pub mod error;
pub mod playground;
pub mod support;
pub mod utils;

use crate::common::error::TestError;
use uuid::Uuid;
extern crate utilities;

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
    access_key: Option<String>,
    secret_key: Option<String>,
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
    pub fn access_key(&self) -> Option<String> {
        self.access_key.clone()
    }
    pub fn secret_key(&self) -> Option<String> {
        self.secret_key.clone()
    }
}

#[allow(dead_code)]
pub fn new_doc_id() -> String {
    format!("test-{}", Uuid::new_v4().to_string())
}
