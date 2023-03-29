use crate::cli::doc_upsert::run_kv_mutations;
use crate::client::KeyValueRequest;
use crate::state::State;
use nu_command::Open;
use nu_engine::CallExt;
use std::sync::{Arc, Mutex};

use crate::cli::error::serialize_error;
use crate::cli::util::convert_nu_value_to_json_value;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct DocImport {
    state: Arc<Mutex<State>>,
}

impl DocImport {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for DocImport {
    fn name(&self) -> &str {
        "doc import"
    }

    fn signature(&self) -> Signature {
        Signature::build("doc import")
            .required(
                "filename",
                SyntaxShape::String,
                "the path to the file containing data to import",
            )
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
            .category(Category::Custom("couchbase".to_string()))
    }

    fn usage(&self) -> &str {
        "Import documents from a file through the data service"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        run_import(self.state.clone(), engine_state, stack, call, input)
    }
}

fn build_req(key: String, value: Vec<u8>, expiry: u32) -> KeyValueRequest {
    KeyValueRequest::Set { key, value, expiry }
}

fn run_import(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let open = Open;
    let data = open.run(engine_state, stack, call, input)?;

    let id_column = call
        .get_flag(engine_state, stack, "id-column")?
        .unwrap_or_else(|| String::from("id"));

    let filtered = data
        .into_iter()
        .filter_map(move |i| {
            let id_column = id_column.clone();

            if let Value::Record { cols, vals, .. } = i {
                let mut id = None;
                let mut content = vec![];
                for (k, v) in cols.iter().zip(vals) {
                    if k.clone() == id_column {
                        id = v.as_string().ok();
                    }

                    content.push(convert_nu_value_to_json_value(&v, span).ok()?);
                }
                if let Some(i) = id {
                    return Some((i, content));
                }
            }
            None
        })
        .collect::<Vec<(String, Vec<serde_json::Value>)>>();

    let mut all_items = vec![];
    for item in filtered {
        let value =
            serde_json::to_vec(&item.1).map_err(|e| serialize_error(e.to_string(), span))?;

        all_items.push((item.0, value));
    }

    let results = run_kv_mutations(state, engine_state, stack, call, span, all_items, build_req)?;

    Ok(Value::List {
        vals: results,
        span: call.head,
    }
    .into_pipeline_data())
}
