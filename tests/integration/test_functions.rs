use crate::tests::*;
use crate::util::TestConfig;
use futures::Future;
use std::pin::Pin;
use std::sync::Arc;

// Sad panda noises
pub fn tests(config: Arc<TestConfig>) -> Vec<TestFn> {
    vec![
        TestFn::new(
            "test_get_a_document",
            Box::pin(doc_get::test_get_a_document(config.clone())),
        ),
        TestFn::new(
            "test_get_a_document_not_found",
            Box::pin(doc_get::test_get_a_document_not_found(config.clone())),
        ),
        TestFn::new(
            "test_upserts_a_document",
            Box::pin(doc_upsert::test_upserts_a_document(config.clone())),
        ),
    ]
}

pub struct TestFn {
    pub name: String,
    pub func: Pin<Box<dyn Future<Output = bool> + Send + 'static>>,
}

impl TestFn {
    pub fn new(
        name: impl Into<String>,
        func: Pin<Box<dyn Future<Output = bool> + Send + 'static>>,
    ) -> Self {
        Self {
            name: name.into(),
            func,
        }
    }
}
