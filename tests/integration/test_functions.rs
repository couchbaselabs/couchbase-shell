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
        TestFn::new(
            "query_advise::test_should_get_advise_with_context",
            Box::pin(query_advise::test_should_get_advise_with_context(
                cluster.clone(),
            )),
        ),
        TestFn::new(
            "query_advise::test_should_get_advise",
            Box::pin(query_advise::test_should_get_advise(cluster.clone())),
        ),
        TestFn::new(
            "query_indexes::test_should_get_indexes_with_context",
            Box::pin(query_indexes::test_should_get_indexes_with_context(
                cluster.clone(),
            )),
        ),
        TestFn::new(
            "query_indexes::test_should_get_indexes",
            Box::pin(query_indexes::test_should_get_indexes(cluster.clone())),
        ),
        TestFn::new(
            "query_indexes::test_should_get_index_definitions",
            Box::pin(query_indexes::test_should_get_index_definitions(
                cluster.clone(),
            )),
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
