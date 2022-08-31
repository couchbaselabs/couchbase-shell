use crate::tests::*;
use crate::ClusterUnderTest;
use futures::Future;
use std::pin::Pin;
use std::sync::Arc;

// Sad panda noises
pub fn tests(cluster: Arc<ClusterUnderTest>) -> Vec<TestFn> {
    vec![
        TestFn::new(
            "doc_get::test_get_a_document",
            Box::pin(doc_get::test_get_a_document(cluster.clone())),
        ),
        TestFn::new(
            "doc_get::test_get_a_document_not_found",
            Box::pin(doc_get::test_get_a_document_not_found(cluster.clone())),
        ),
        TestFn::new(
            "doc_upsert::test_upserts_a_document",
            Box::pin(doc_upsert::test_upserts_a_document(cluster.clone())),
        ),
        TestFn::new(
            "query::test_should_send_context_with_a_query",
            Box::pin(query::test_should_send_context_with_a_query(
                cluster.clone(),
            )),
        ),
        TestFn::new(
            "query::test_should_execute_a_query",
            Box::pin(query::test_should_execute_a_query(cluster.clone())),
        ),
        TestFn::new(
            "query::test_should_fetch_meta",
            Box::pin(query::test_should_fetch_meta(cluster.clone())),
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
