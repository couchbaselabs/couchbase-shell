use crate::state::State;

use async_openai::config::OpenAIConfig;
use async_openai::{types::CreateEmbeddingRequestArgs, Client};
use std::str;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;

use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::Command;
use nu_protocol::engine::{EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};
use std::time::SystemTime;
use tiktoken_rs::p50k_base;

// The maximum tokens per minute for the free tier of the embedding model in use
// Documented value is 150000 but that resulted in too large batches, maybe the local tokeniser being used is wrong
const MAX_TOKENS: usize = 100000;

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
            // TO DO - add batch and dim as named args
            .required(
                "field",
                SyntaxShape::String,
                "the field from which the vector is generated",
            )
            .named(
                "res_field",
                SyntaxShape::Int,
                "name of field to store resulting embedding in",
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
    _state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;

    let input: Vec<Value> = call.req(engine_state, stack, 0)?;
    let field: String = call.req(engine_state, stack, 1)?;

    // Read the specified field from each doc and add to the source
    let mut source: Vec<String> = vec![];
    for i in &input {
        // Record is a tuple of arrays, the first is the field names, and the second the correcponding values
        let record = i.as_record().unwrap();

        let index = match record.0.iter().position(|r| *r == field) {
            Some(i) => i,
            None => {
                return Err(ShellError::GenericError(
                    format!("Could not find field ({})", field),
                    "".to_string(),
                    None,
                    None,
                    Vec::new(),
                ));
            }
        };
        source.push(record.1[index].as_string().unwrap())
    }

    let key = match engine_state.get_env_var("OPENAI_API_KEY") {
        Some(k) => match k.as_string() {
            Ok(k) => k,
            Err(e) => {
                return Err(ShellError::GenericError(
                    format!("could not read OPENAI_API_KEY env var as a string: {}", e),
                    "".to_string(),
                    None,
                    None,
                    Vec::new(),
                ));
            }
        },
        None => {
            return Err(ShellError::GenericError(
                "Please specify API key using: \"$env.OPENAI_API_KEY = <YOUR API KEY>\""
                    .to_string(),
                "".to_string(),
                None,
                None,
                Vec::new(),
            ));
        }
    };
    let client =
        Client::with_config(async_openai::config::OpenAIConfig::default().with_api_key(key));

    // TESTING
    println!("Strings to embed: {:?}", source.len());

    // Calculate the total tokens for all the text that we want to embed
    // let mut text = "".to_string();
    // for txt in &source {
    //     text.push_str(&txt);
    //     text.push_str(&" ".to_string());
    // }
    let bpe = p50k_base().unwrap();
    let tokens = bpe.encode_with_special_tokens(&source.join(" "));
    // TESTING
    println!("{}", tokens.len());

    // Add 1 to account for int div
    let num_batches = (tokens.len() / MAX_TOKENS) + 1;
    let batch_size = source.len() / num_batches;

    let mut batches: Vec<Vec<String>> = Vec::with_capacity(source.len() / batch_size);
    if num_batches == 1 {
        batches.push(source.to_vec());
    } else {
        let mut lower = 0;
        let mut upper = batch_size;
        while lower < source.len() {
            batches.push(source[lower..=upper].to_vec());
            lower = upper + 1;
            upper += batch_size;

            if upper > source.len() {
                upper = source.len() - 1;
            }
        }
    };

    // Split chunks into batches
    let mut records = vec![];
    let mut count = 0;

    let start = SystemTime::now();
    let rt = Runtime::new().unwrap();
    for batch in batches {
        println!("Getting results for batch with length {:?}", batch.len());
        let batch_start = SystemTime::now();
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

        for embedding in response.data {
            let record = input[count].as_record().unwrap();
            let mut res_string: Vec<String> = record.0.to_vec();
            res_string.push(format!("{}Vector", field));

            let mut res_vals: Vec<Value> = record.1.to_vec();
            res_vals.push(Value::List {
                span,
                vals: embedding
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
                cols: res_string,
                vals: res_vals,
            });

            count += 1;
        }

        let now = SystemTime::now();
        let difference = now.duration_since(batch_start);
        println!("- Duration: {:?}", difference.unwrap());
    }

    let total_time = SystemTime::now().duration_since(start);
    print!("Total Duration: {:?}\n", total_time.unwrap());

    Ok(Value::List {
        span,
        vals: records,
    }
    .into_pipeline_data())
}
