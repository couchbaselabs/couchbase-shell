//! The `doc insert` command performs a KV insert operation.

use crate::cli::doc_upsert::run_kv_store_ops;
use crate::client::KeyValueRequest;
use crate::state::State;
use std::sync::{Arc, Mutex};

use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct DocInsert {
    state: Arc<Mutex<State>>,
}

impl DocInsert {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for DocInsert {
    fn name(&self) -> &str {
        "doc insert"
    }

    fn signature(&self) -> Signature {
        Signature::build("doc insert")
            .optional("id", SyntaxShape::String, "the document id")
            .optional("content", SyntaxShape::Any, "the document content")
            .named(
                "id-column",
                SyntaxShape::String,
                "the name of the id column if used with an input stream",
                None,
            )
            .named(
                "bucket",
                SyntaxShape::String,
                "the name of the bucket",
                None,
            )
            .named(
                "content-column",
                SyntaxShape::String,
                "the name of the content column if used with an input stream",
                None,
            )
            .named(
                "expiry",
                SyntaxShape::Number,
                "the expiry for the documents in seconds, or absolute",
                None,
            )
            .named("scope", SyntaxShape::String, "the name of the scope", None)
            .named(
                "collection",
                SyntaxShape::String,
                "the name of the collection",
                None,
            )
            .named(
                "clusters",
                SyntaxShape::String,
                "the clusters which should be contacted",
                None,
            )
            .named(
                "batch-size",
                SyntaxShape::Number,
                "the maximum number of items to batch send at a time",
                None,
            )
            .switch("halt-on-error", "halt on any errors", Some('e'))
            .category(Category::Custom("couchbase".to_string()))
    }

    fn usage(&self) -> &str {
        "Insert a document through the data service"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        run_insert(self.state.clone(), engine_state, stack, call, input)
    }
}

fn build_req(key: String, value: Vec<u8>, expiry: u32) -> KeyValueRequest {
    KeyValueRequest::Insert { key, value, expiry }
}

fn run_insert(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let results = run_kv_store_ops(state, engine_state, stack, call, input, build_req)?;

    Ok(Value::List {
        vals: results,
        internal_span: call.head,
    }
    .into_pipeline_data())
}
