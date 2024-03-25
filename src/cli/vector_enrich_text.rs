use crate::cli::util::read_openai_api_key;
use crate::state::State;
use crate::CtrlcFuture;
use crate::OpenAIClient;
use log::{debug, info};
use nu_protocol::Example;
use std::convert::TryFrom;
use std::fs;
use std::str;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use tokio::runtime::Runtime;
use tokio::select;
use uuid::Uuid;

use crate::cli::llm_client::LLMClients;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::Command;
use nu_protocol::engine::{EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct VectorEnrichText {
    state: Arc<Mutex<State>>,
}

impl VectorEnrichText {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for VectorEnrichText {
    fn name(&self) -> &str {
        "vector enrich-text"
    }

    fn signature(&self) -> Signature {
        Signature::build("vector enrich-text")
            .named(
                "chunk",
                SyntaxShape::Int,
                "length of the data chunks to embed (default 1024)",
                None,
            )
            .named(
                "dimension",
                SyntaxShape::Int,
                "dimension of the resulting embeddings (default 128)",
                None,
            )
            .named(
                "maxTokens",
                SyntaxShape::Int,
                "the token per minute limit with 'text-embedding-3-small' for your API key",
                None,
            )
            .category(Category::Custom("couchbase".to_string()))
    }

    fn usage(&self) -> &str {
        "Chunks text and generates vector indexable json documents on the chunks"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        vector_enrich_text(self.state.clone(), engine_state, stack, call, input)
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Retrieves an embedding for a plain text string",
                example: "\"embed this for me\" | vector enrich-text",
                result: None,
            },
            Example {
                description:
                    "Chunks longer text from file and retrieves the embedding for the chunks",
                example: "open ./some-text.txt | vector enrich-text",
                result: None,
            },
            Example {
                description:
                    "Chunks text from all files in the current directory, retrieves embeddings \n  and uploads the vector docs to couchbase",
                example: "ls | vector enrich-text | doc upsert",
                result: None,
            },
        ]
    }
}

fn vector_enrich_text(
    _state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;

    let chunk_len = match call.get_flag::<usize>(engine_state, stack, "chunk")? {
        Some(l) => l,
        None => 1024,
    };

    let dim = match call.get_flag::<i64>(engine_state, stack, "dimension")? {
        Some(d) => u32::try_from(d).ok().unwrap(),
        None => 128,
    };

    let max_tokens: Option<usize> = call.get_flag::<usize>(engine_state, stack, "maxTokens")?;

    let mut chunks: Vec<String> = Vec::new();
    match input.into_value(span) {
        Value::List { vals, span: _span } => {
            for v in vals {
                let rec = match v.as_record() {
                    Ok(r) => r,
                    Err(e) => {
                        return Err(ShellError::GenericError(
                            "Could not parse list of files".to_string(),
                            "".to_string(),
                            None,
                            None,
                            vec![e],
                        ));
                    }
                };

                let index = match rec.0.iter().position(|r: &String| *r == "name") {
                    Some(i) => i,
                    None => {
                        return Err(ShellError::GenericError(
                            "Could not parse list of files".to_string(),
                            "".to_string(),
                            None,
                            None,
                            Vec::new(),
                        ));
                    }
                };

                let file = rec.1[index].as_string().unwrap();
                let contents = match fs::read_to_string(file.clone()) {
                    Ok(c) => c,
                    Err(e) => {
                        return Err(ShellError::GenericError(
                            format!("Error parsing file {}: {}", file, e),
                            "".to_string(),
                            None,
                            None,
                            Vec::new(),
                        ))
                    }
                };

                let file_chunks = &mut chunk_text(contents, chunk_len);
                chunks.append(file_chunks);
            }
        }
        Value::String { val, span: _ } => {
            chunks = chunk_text(val, chunk_len);
        }
        _ => {
            return Err(ShellError::GenericError(
                "Piped input must be a string or list of files from ls".to_string(),
                "".to_string(),
                None,
                None,
                Vec::new(),
            ));
        }
    };

    let key = read_openai_api_key(engine_state)?;
    let client = LLMClients::OpenAI(OpenAIClient::new(key, max_tokens));

    let mut results: Vec<Value> = Vec::new();
    let batches = client.batch_chunks(chunks);

    let start = SystemTime::now();
    for (i, batch) in batches.iter().enumerate() {
        let batch_start = SystemTime::now();
        info!("Embedding batch {:?}/{} ", i + 1, batches.len());

        let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();
        let ctrl_c_fut = CtrlcFuture::new(ctrl_c);
        let rt = Runtime::new().unwrap();
        let embeddings = match rt.block_on(async {
            select! {
                result = client.embed(batch, dim) => {
                    match result {
                        Ok(r) => Ok(r),
                        Err(e) => Err(ShellError::GenericError(
                            format!("failed to execute request: {}", e),
                            "".to_string(),
                            None,
                            None,
                            Vec::new(),
                        ))
                    }
                },
                () = ctrl_c_fut =>
                     Err(ShellError::GenericError(
                   "Request cancelled".to_string(),
                    "".to_string(),
                    None,
                    None,
                    Vec::new(),
                )),
            }
        }) {
            Ok(r) => r,
            Err(e) => {
                return Err(e);
            }
        };

        for (i, chunk) in batch.iter().enumerate() {
            let vector = embeddings[i]
                .iter()
                .map(|x| Value::Float {
                    val: *x as f64,
                    span,
                })
                .collect::<Vec<Value>>();

            let mut uuid = Uuid::new_v4().to_string();
            uuid.truncate(6);
            let vector_doc = Value::Record {
                cols: vec!["id".to_string(), "content".to_string()],
                vals: vec![
                    Value::String {
                        val: format!("vector-{}", uuid),
                        span,
                    },
                    Value::Record {
                        cols: vec!["text".to_string(), "vector".to_string()],
                        vals: vec![
                            Value::String {
                                val: chunk.to_string(),
                                span,
                            },
                            Value::List { vals: vector, span },
                        ],
                        span,
                    },
                ],
                span,
            };

            results.push(vector_doc);
        }

        let now = SystemTime::now();
        let difference = now.duration_since(batch_start);
        debug!("- Duration: {:?}", difference.unwrap());
    }

    let total_time = SystemTime::now().duration_since(start);
    debug!("\nTotal Duration: {:?}", total_time.unwrap());

    Ok(Value::List {
        span,
        vals: results,
    }
    .into_pipeline_data())
}

fn chunk_text(text: String, chunk_len: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut iter = text.chars();
    let mut pos = 0;
    while pos < text.len() {
        let mut len = 0;
        for ch in iter.by_ref().take(chunk_len) {
            len += ch.len_utf8();
        }
        let chunk = &text[pos..pos + len];
        chunks.push(chunk.to_string());
        pos += len;
    }
    chunks
}
