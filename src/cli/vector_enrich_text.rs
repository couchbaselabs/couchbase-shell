use crate::state::State;
use crate::CtrlcFuture;
use log::debug;
use nu_protocol::Example;
use nu_protocol::Record;
use std::fs;
use std::str;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use tokio::runtime::Runtime;
use tokio::select;
use uuid::Uuid;

use crate::client::LLMClients;
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
            .optional("text", SyntaxShape::String, "the text to be embedded")
            .named(
                "model",
                SyntaxShape::String,
                "the model to generate the embeddings with",
                None,
            )
            .named(
                "chunk",
                SyntaxShape::Int,
                "length of the data chunks to embed (default 1024)",
                None,
            )
            .named(
                "dimension",
                SyntaxShape::Int,
                "dimension of the resulting embeddings",
                None,
            )
            .named(
                "maxTokens",
                SyntaxShape::Int,
                "the token per minute limit for the provider/model",
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
                description: "Retrieves an embedding from a plain text string",
                example: "vector enrich-text \"embed this for me\" --model amazon.titan-embed-text-v2:0",
                result: None
            },
            Example {
                description: "Retrieves an embedding for a plain text string from pipeline data",
                example: "\"embed this for me\" | vector enrich-text --model models/text-embedding-004",
                result: None,
            },
            Example {
                description:
                    "Chunks longer text from file and retrieves the embedding for the chunks",
                example: "open ./some-text.txt | vector enrich-text --model amazon.titan-embed-text-v1",
                result: None,
            },
            Example {
                description:
                    "Chunks text from all files in the current directory, retrieves embeddings \n  and uploads the vector docs to couchbase",
                example: "ls | vector enrich-text --model models/embedding-001 | doc upsert",
                result: None,
            },
        ]
    }
}

fn vector_enrich_text(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;

    let dim = call.get_flag::<usize>(engine_state, stack, "dimension")?;

    let max_tokens: Option<usize> = call.get_flag::<usize>(engine_state, stack, "maxTokens")?;

    let model = match call.get_flag::<String>(engine_state, stack, "model")? {
        Some(m) => m,
        None => {
            let guard = state.lock().unwrap();
            let model = match guard.llm() {
                Some(m) => match m.model() {
                    Some(m) => m,
                    None => {
                        return Err(ShellError::GenericError {
                            error: "no model provided".to_string(),
                            msg: "".to_string(),
                            span: Some(span),
                            help: Some(
                                "supply the model in the config file or using the --model flag"
                                    .to_string(),
                            ),
                            inner: Vec::new(),
                        });
                    }
                },
                None => {
                    return Err(ShellError::GenericError {
                        error: "no llm defined in config file".to_string(),
                        msg: "".to_string(),
                        span: Some(span),
                        help: None,
                        inner: Vec::new(),
                    });
                }
            };
            model
        }
    };

    let client = LLMClients::new(state, max_tokens)?;

    let mut results: Vec<Value> = Vec::new();
    let chunks = chunks_from_input(input, call, engine_state, stack)?;
    let batches = client.batch_chunks(chunks);

    let start = SystemTime::now();
    for (i, batch) in batches.iter().enumerate() {
        let batch_start = SystemTime::now();
        println!("Embedding batch {:?}/{} ", i + 1, batches.len());

        let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();
        let ctrl_c_fut = CtrlcFuture::new(ctrl_c);
        let rt = Runtime::new().unwrap();
        let embeddings = match rt.block_on(async {
            select! {
                result = client.embed(batch, dim, model.clone()) => {
                    match result {
                        Ok(r) => Ok(r),
                        Err(e) => Err(e)
                    }
                },
                () = ctrl_c_fut =>
                     Err(ShellError::GenericError{
                   error: "Request cancelled".to_string(),
                    msg: "".to_string(),
                    span: None,
                    help: None,
                    inner: Vec::new(),
            }),
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
                    internal_span: span,
                })
                .collect::<Vec<Value>>();

            let mut uuid = Uuid::new_v4().to_string();
            uuid.truncate(6);

            let cols = vec!["text".to_string(), "vector".to_string()];
            let vals = vec![
                Value::String {
                    val: chunk.to_string(),
                    internal_span: span,
                },
                Value::List {
                    vals: vector,
                    internal_span: span,
                },
            ];
            let content = Value::Record {
                val: Box::new(Record::from_raw_cols_vals(cols, vals, span, span).unwrap()),
                internal_span: span,
            };

            let cols = vec!["id".to_string(), "content".to_string()];
            let vals = vec![
                Value::String {
                    val: format!("vector-{}", uuid),
                    internal_span: span,
                },
                content,
            ];
            let vector_doc = Value::Record {
                val: Box::new(Record::from_raw_cols_vals(cols, vals, span, span).unwrap()),
                internal_span: span,
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
        internal_span: span,
        vals: results,
    }
    .into_pipeline_data())
}

fn chunks_from_input(
    input: PipelineData,
    call: &Call,
    engine_state: &EngineState,
    stack: &mut Stack,
) -> Result<Vec<String>, ShellError> {
    let span = call.head;
    let mut chunks: Vec<String> = Vec::new();

    let chunk_len = match call.get_flag::<usize>(engine_state, stack, "chunk")? {
        Some(l) => l,
        None => 1024,
    };

    match input.into_value(span) {
        Value::List { vals, .. } => {
            for v in vals {
                let rec = match v.as_record() {
                    Ok(r) => r,
                    Err(e) => {
                        return Err(ShellError::GenericError {
                            error: "Could not parse list of files".to_string(),
                            msg: "".to_string(),
                            span: None,
                            help: None,
                            inner: vec![e],
                        });
                    }
                };

                let file = match rec.get("name") {
                    Some(f) => f.as_str().unwrap(),
                    None => {
                        return Err(ShellError::GenericError {
                            error: "Could not parse list of files".to_string(),
                            msg: "".to_string(),
                            span: None,
                            help: None,
                            inner: Vec::new(),
                        });
                    }
                };

                let contents = match fs::read_to_string(file) {
                    Ok(c) => c,
                    Err(e) => {
                        return Err(ShellError::GenericError {
                            error: format!("Error parsing fimsg: le {}: {}", file, e),
                            msg: "".to_string(),
                            span: None,
                            help: None,
                            inner: Vec::new(),
                        })
                    }
                };

                let file_chunks = &mut chunk_text(contents, chunk_len);
                chunks.append(file_chunks);
            }
        }
        Value::String { val, .. } => {
            chunks = chunk_text(val, chunk_len);
        }
        Value::Nothing { .. } => {
            let text: String = match call.opt(engine_state, stack, 0)? {
                Some(t) => t,
                None => {
                    return Err(ShellError::GenericError {
                        error: "Please supply source text as shown in examples`".to_string(),
                        msg: "".to_string(),
                        span: None,
                        help: None,
                        inner: Vec::new(),
                    });
                }
            };
            chunks = chunk_text(text, chunk_len);
        }
        _ => {
            return Err(ShellError::GenericError {
                error: "Please supply source text as shown in examples`".to_string(),
                msg: "".to_string(),
                span: None,
                help: None,
                inner: Vec::new(),
            });
        }
    };

    Ok(chunks)
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
