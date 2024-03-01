use crate::state::State;
use log::debug;
use std::time::SystemTime;

use crate::cli::util::read_openai_api_key;
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

const MAX_FREE_TIER_TOKENS: usize = 50000;

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
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;

    let test_string = "This is a test with a \n new line";
    let has_newline = test_string.find("\n");
    if !has_newline.is_none() {
        println!("NEW LINE FOUND")
    }

    let input: Vec<Value> = call.req(engine_state, stack, 0)?;
    let field: String = call.req(engine_state, stack, 1)?;

    let mut average_len = 0;
    let mut max_len = 0;
    let mut max_tokens = 0;

    let mut source_posts: Vec<String> = vec![];
    let mut source_records: Vec<&Value> = vec![];
    for i in &input {
        // println!("{:?}", i);
        // Is this how we want to take the input to this command?
        let record = i.as_record().unwrap();

        // Find the index of the named field, handle the field missing here
        let index = record.0.iter().position(|r| *r == field).unwrap();

        let post_string = record.1[index].as_string().unwrap();
        if post_string != "" {
            let bpe = p50k_base().unwrap();
            let tokens = bpe.encode_with_special_tokens(post_string.as_str());
            if tokens.len() < 8192 {
                source_posts.push(record.1[index].as_string().unwrap());
                source_records.push(i);
                average_len += record.1[index].as_string().unwrap().len();
            } else {
                println!("TOO MANY TOKENS");
            }

            // if record.1[index].as_string().unwrap().len() > max_len {
            //  max_len = record.1[index].as_string().unwrap().len();

            // }
        } else {
            println!("EMPTY STRING FOUND")
        }

        // let bpe = p50k_base().unwrap();
        // let tokens = bpe.encode_with_special_tokens(post_string.as_str());
        // if tokens
    }

    println!("AVERAGE LEN: {:?}", average_len / source_posts.len());
    println!("Max tokens: {:?}", max_tokens);

    println!("INPUT {:?}", input.len());
    println!("POSTS {:?}", source_posts.len());
    println!("RECS {:?}", source_records.len());

    // TO DO - batch requests if source is sufficiently long

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

    // Split up the source slice to be sent in various requests

    // TO DO - have the user supply this
    println!("Strings to embed: {:?}", source_posts.len());

    let bpe = p50k_base().unwrap();
    let tokens = bpe.encode_with_special_tokens(&source_posts.join(" "));

    println!("Total tokens: {:?}\n", tokens.len());
    let num_batches = (tokens.len() / MAX_FREE_TIER_TOKENS) + 1;
    let batch_size = source_posts.len() / num_batches;

    println!("BATCH SIZE: {:?}", batch_size);

    let mut batches: Vec<Vec<String>> = Vec::new();
    if num_batches == 1 {
        batches.push(source_posts.to_vec());
    } else {
        let mut lower = 0;
        let mut upper = batch_size;
        while lower < source_posts.len() {
            let bpe = p50k_base().unwrap();
            let tokens =
                bpe.encode_with_special_tokens(&source_posts[lower..=upper].to_vec().join(" "));
            // println!("- Tokens: {:?}", tokens.len());

            if tokens.len() > MAX_FREE_TIER_TOKENS {
                upper = upper - batch_size / 2;
                let bpe = p50k_base().unwrap();
                let tokens =
                    bpe.encode_with_special_tokens(&source_posts[lower..=upper].to_vec().join(" "));
                // println!("- New Tokens: {:?}", tokens.len());
            }

            batches.push(source_posts[lower..=upper].to_vec());
            lower = upper + 1;
            upper += batch_size;

            if upper >= source_posts.len() {
                upper = source_posts.len() - 1;
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

        let mut sub_count = 0;
        while sub_count < batch.len() {
            let record = source_records[count].as_record().unwrap(); //.1[0].as_record().unwrap();

            let mut res_string: Vec<String> = record.0.to_vec();
            res_string.push(format!("{}Vector", field));

            let mut res_vals: Vec<Value> = record.1.to_vec();
            let mut temp: Vec<Value> = response.data[sub_count]
                .embedding
                .clone()
                .iter()
                .map(|&e| Value::Float {
                    val: e as f64,
                    span,
                })
                .collect();

            res_vals.push(Value::List { span, vals: temp });

            records.push(Value::Record {
                span,
                cols: res_string,
                vals: res_vals,
            });

            count += 1;
            sub_count += 1;
        }

        let now = SystemTime::now();
        let difference = now.duration_since(batch_start);
        println!("- Duration: {:?}", difference.unwrap());
        // for i in &input {
        //     let record = i.as_record().unwrap(); //.1[0].as_record().unwrap();

        //     let mut res_string: Vec<String> = record.0.to_vec();
        //     res_string.push(format!("{}Vector", field));

        //     let mut res_vals: Vec<Value> = record.1.to_vec();
        //     res_vals.push(Value::String {
        //         span,
        //         val: format!("{:?}", response.data[count].embedding.clone()),
        //     });

        //     records.push(Value::Record {
        //         span,
        //         cols: res_string,
        //         vals: res_vals,
        //     });

        //     count += 1;
        // }
    }
    let total_time = SystemTime::now().duration_since(start);
    println!("\nTotal Duration: {:?}", total_time.unwrap());

    Ok(Value::List {
        span,
        vals: records,
    }
    .into_pipeline_data())
}
