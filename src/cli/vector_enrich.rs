use crate::state::State;
use log::debug;
use std::cmp;
use std::time::SystemTime;

use crate::cli::util::{read_openai_api_key, MAX_FREE_TIER_TOKENS};
use async_openai::{types::CreateEmbeddingRequestArgs, Client};
use std::io::Write;
use std::str;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;

use nu_engine::CallExt;
use std::io::stdout;
use tiktoken_rs::p50k_base;

use nu_protocol::ast::Call;
use nu_protocol::engine::Command;
use nu_protocol::engine::{EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct VectorEnrich {
    state: Arc<Mutex<State>>,
}

impl VectorEnrich {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for VectorEnrich {
    fn name(&self) -> &str {
        "vector enrich"
    }

    fn signature(&self) -> Signature {
        Signature::build("vector enrich")
            .required("input", SyntaxShape::Any, "the json data to be enriched")
            .required(
                "field",
                SyntaxShape::String,
                "the field from which the vector is generated",
            )
            // Is this necessary?
            .named(
                "res_field",
                SyntaxShape::Int,
                "name of field to store resulting embedding in",
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
        "Enriches given JSON with embeddings of selected field"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        vector_enrich(self.state.clone(), engine_state, stack, call, input)
    }
}

fn vector_enrich(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;

    let input: Vec<Value> = call.req(engine_state, stack, 0)?;
    let field: String = call.req(engine_state, stack, 1)?;

    let mut field_contents: Vec<String> = vec![];
    let mut input_records: Vec<(&[String], &[Value])> = vec![];

    // The enrichment will currently fail if any of the post_strings are longer than 8192 tokens, due to embeddgin limits
    for i in &input {
        let record = match i.as_record() {
            Ok(r) => r,
            Err(e) => {
                return Err(ShellError::GenericError(
                    "Input must be a list of json records".to_string(),
                    "".to_string(),
                    None,
                    None,
                    vec![e],
                ));
            }
        };

        let index = match record.0.iter().position(|r: &String| *r == field) {
            Some(i) => i,
            None => {
                return Err(ShellError::GenericError(
                    format!("The field '{}' must be present in all input records", field),
                    "".to_string(),
                    None,
                    None,
                    Vec::new(),
                ));
            }
        };

        let content = match record.1[index].as_string() {
            Ok(c) => c,
            Err(e) => {
                return Err(ShellError::GenericError(
                    "".to_string(),
                    "".to_string(),
                    None,
                    None,
                    vec![e],
                ));
            }
        };

        //The API will return an error on empty strings
        if content != "" {
            field_contents.push(content);
            input_records.push(record);
        }
    }

    // For each string of text in source
    // Batch into chunks and get the embedding
    let rt = Runtime::new().unwrap();
    let key = match read_openai_api_key(engine_state) {
        Ok(k) => k,
        Err(e) => {
            return Err(e);
        }
    };

    let client =
        Client::with_config(async_openai::config::OpenAIConfig::default().with_api_key(key));

    // TO DO - have the user supply this
    println!("Strings to embed: {:?}", field_contents.len());
    let bpe = p50k_base().unwrap();
    let tokens = bpe.encode_with_special_tokens(&field_contents.join(" "));

    println!("Total tokens: {:?}\n", tokens.len());
    let num_batches = (tokens.len() / (MAX_FREE_TIER_TOKENS)) + 1;

    //Regardless of token limit OpenAI's API can only accept arrays of strings up to 2048 in length
    let batch_size = cmp::min(2047, field_contents.len() / num_batches);

    println!("BATCH SIZE: {:?}", batch_size);
    let mut batches: Vec<Vec<String>> = Vec::new();
    if num_batches == 1 {
        batches.push(field_contents.to_vec());
    } else {
        let mut lower = 0;
        let mut upper = batch_size;
        while lower < field_contents.len() {
            let bpe = p50k_base().unwrap();
            let tokens =
                bpe.encode_with_special_tokens(&field_contents[lower..=upper].to_vec().join(" "));
            println!("- Tokens: {:?}", tokens.len());

            if tokens.len() > MAX_FREE_TIER_TOKENS {
                upper = upper - batch_size / 2;
                let bpe = p50k_base().unwrap();
                let tokens = bpe
                    .encode_with_special_tokens(&field_contents[lower..=upper].to_vec().join(" "));
                println!("- New Tokens: {:?}", tokens.len());
            }

            batches.push(field_contents[lower..=upper].to_vec());
            lower = upper + 1;
            upper += batch_size;

            if upper >= field_contents.len() {
                upper = field_contents.len() - 1;
            }
        }
    };

    println!("Batches: {:?}", batches.len());
    println!("Length of batches: ");

    let mut records = vec![];
    let start = SystemTime::now();
    let mut count = 0;
    for (i, batch) in batches.iter().enumerate() {
        let batch_start = SystemTime::now();
        print!("\rEmbedding batch {:?}/{} ", i + 1, batches.len());
        stdout().flush().unwrap();

        let bpe = p50k_base().unwrap();
        let tokens = bpe.encode_with_special_tokens(&batch.join(" "));
        println!("- Tokens: {:?}", tokens.len());

        // if log::log_enabled!(log::Level::Debug) {
        //     println!("");
        //     let bpe = p50k_base().unwrap();
        //     let tokens = bpe.encode_with_special_tokens(&batch.join(" "));
        //     debug!("- Tokens: {:?}", tokens.len());
        // }

        let request = CreateEmbeddingRequestArgs::default()
            .model("text-embedding-3-small")
            .dimensions(128 as u32)
            .input(batch.clone())
            .build()
            .unwrap();

        let response = match rt.block_on(async { client.embeddings().create(request).await }) {
            Ok(r) => r,
            Err(e) => {
                println!("");
                return Err(ShellError::GenericError(
                    format!("failed to execute request: {}", e),
                    "".to_string(),
                    None,
                    None,
                    Vec::new(),
                ));
            }
        };

        for (i, _) in batch.iter().enumerate() {
            // while sub_count < batch.len() {
            let mut res_keys: Vec<String> = input_records[count].0.to_vec();
            res_keys.push(format!("{}Vector", field));

            let mut res_vals: Vec<Value> = input_records[count].1.to_vec();
            res_vals.push(Value::List {
                span,
                vals: response.data[i]
                    .embedding
                    .clone()
                    .iter()
                    .map(|&e| Value::Float {
                        val: e as f64,
                        span,
                    })
                    .collect(),
            });

            records.push(Value::Record {
                span,
                cols: res_keys,
                vals: res_vals,
            });

            count += 1;
        }

        let now = SystemTime::now();
        let difference = now.duration_since(batch_start);
        println!("- Duration: {:?}", difference.unwrap());
    }
    let total_time = SystemTime::now().duration_since(start);
    println!("\nTotal Duration: {:?}", total_time.unwrap());

    Ok(Value::List {
        span,
        vals: records,
    }
    .into_pipeline_data())
}
