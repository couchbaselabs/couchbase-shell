use crate::cli::util::{convert_json_value_to_nu_value, read_openai_api_key};
use crate::state::State;
use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::fs;
use std::io::stdout;

use std::io::Write;
use tiktoken_rs::p50k_base;

use std::str;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use tokio::runtime::Runtime;

use async_openai::{types::CreateEmbeddingRequestArgs, Client};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::Command;
use nu_protocol::engine::{EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};

const MAX_FREE_TIER_TOKENS: usize = 150000;

#[derive(Clone)]
pub struct VectorGenerate {
    state: Arc<Mutex<State>>,
}

impl VectorGenerate {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for VectorGenerate {
    fn name(&self) -> &str {
        "vector generate"
    }

    fn signature(&self) -> Signature {
        Signature::build("vector generate")
            .optional(
                "input",
                SyntaxShape::Any,
                "string to generate embedding from.",
            )
            .named(
                "files",
                SyntaxShape::List(Box::new(SyntaxShape::String)),
                "the path of the file(s) from which to generate embeddings.",
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
        "Generates vector embeddings from a given data set"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        vector_generate(self.state.clone(), engine_state, stack, call, input)
    }
}

#[derive(Serialize, Deserialize)]
struct VectorDoc {
    id: String,
    text: String,
    vector: Vec<f32>,
}

fn vector_generate(
    _state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let rt = Runtime::new().unwrap();

    let input: Option<String> = call.opt(engine_state, stack, 0)?;
    let files: Option<Vec<String>> = call.get_flag(engine_state, stack, "files")?;

    if input.is_none() && files.is_none() {
        return Err(ShellError::GenericError(
            "Supply input text or list of files".to_string(),
            "".to_string(),
            None,
            None,
            Vec::new(),
        ));
    }

    if !input.is_none() && !files.is_none() {
        return Err(ShellError::GenericError(
            "Supply either text or files paths, not both".to_string(),
            "".to_string(),
            None,
            None,
            Vec::new(),
        ));
    }

    let key = match read_openai_api_key(engine_state) {
        Ok(k) => k,
        Err(e) => {
            return Err(e);
        }
    };

    let chunk_len = match call.get_flag::<usize>(engine_state, stack, "chunk")? {
        Some(l) => l,
        None => 1024,
    };

    let dim = match call.get_flag::<i64>(engine_state, stack, "dimension")? {
        Some(d) => u32::try_from(d).ok().unwrap(),
        None => 128,
    };

    let max_tokens: usize = match call.get_flag::<usize>(engine_state, stack, "maxTokens")? {
        Some(t) => t,
        None => MAX_FREE_TIER_TOKENS,
    };

    let mut chunks = Vec::new();
    let mut batches: Vec<Vec<String>> = Vec::new();
    let mut results: Vec<Value> = Vec::new();

    if !input.is_none() {
        chunks.push(input.unwrap());
        batches.push(chunks.to_vec());
    } else if !files.is_none() {
        for file in files.unwrap() {
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

            let mut iter = contents.chars();
            let mut pos = 0;
            while pos < contents.len() {
                let mut len = 0;
                for ch in iter.by_ref().take(chunk_len) {
                    len += ch.len_utf8();
                }
                let chunk = &contents[pos..pos + len];
                chunks.push(chunk.to_string());
                pos += len;
            }
        }

        let bpe = p50k_base().unwrap();
        let tokens = bpe.encode_with_special_tokens(&chunks.join(" "));

        debug!("Total tokens: {:?}\n", tokens.len());

        let num_batches = (tokens.len() / max_tokens) + 1;
        let batch_size = chunks.len() / num_batches;
        if num_batches == 1 {
            batches.push(chunks.to_vec());
        } else {
            let mut lower = 0;
            let mut upper = batch_size;
            while lower < chunks.len() {
                batches.push(chunks[lower..=upper].to_vec());
                lower = upper + 1;
                upper += batch_size;

                if upper >= chunks.len() {
                    upper = chunks.len() - 1;
                }
            }
        };
    }

    let client =
        Client::with_config(async_openai::config::OpenAIConfig::default().with_api_key(key));

    let mut vector_count = 0;
    let start = SystemTime::now();
    for (i, batch) in batches.iter().enumerate() {
        let batch_start = SystemTime::now();
        info!("Embedding batch {:?}/{} ", i + 1, batches.len());

        if log::log_enabled!(log::Level::Debug) {
            let bpe = p50k_base().unwrap();
            let tokens = bpe.encode_with_special_tokens(&batch.join(" "));
            debug!("- Tokens: {:?}", tokens.len());
        }

        let request = CreateEmbeddingRequestArgs::default()
            .model("text-embedding-3-small")
            .dimensions(dim)
            .input(batch.clone())
            .build()
            .unwrap();

        let response = match rt.block_on(async { client.embeddings().create(request).await }) {
            Ok(r) => r,
            Err(e) => {
                return Err(ShellError::GenericError(
                    format!("failed to execute request: {}", e),
                    "".to_string(),
                    None,
                    None,
                    Vec::new(),
                ));
            }
        };

        for (i, chunk) in batch.iter().enumerate() {
            let v_doc = VectorDoc {
                id: format!("vector{:?}", vector_count),
                text: chunk.to_string(),
                vector: response.data[i].embedding.clone(),
            };

            let value: serde_json::Value =
                serde_json::from_str(&serde_json::to_string(&v_doc).unwrap()).unwrap();
            results.push(convert_json_value_to_nu_value(&value, span).unwrap());

            vector_count += 1;
        }

        let now = SystemTime::now();
        let difference = now.duration_since(batch_start);
        debug!("- Duration: {:?}", difference.unwrap());
    }

    let total_time = SystemTime::now().duration_since(start);
    println!("\nTotal Duration: {:?}", total_time.unwrap());

    Ok(Value::List {
        span,
        vals: results,
    }
    .into_pipeline_data())
}
