use crate::state::State;
use log::{debug, info};
use std::cmp;
use std::time::SystemTime;

use crate::cli::util::read_openai_api_key;
use crate::CtrlcFuture;
use async_openai::{types::CreateEmbeddingRequestArgs, Client};
use std::str;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;
use tokio::select;

use nu_engine::CallExt;
use tiktoken_rs::p50k_base;

use nu_protocol::ast::Call;
use nu_protocol::engine::Command;
use nu_protocol::engine::{EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};

const MAX_FREE_TIER_TOKENS: usize = 150000;

#[derive(Clone)]
pub struct VectorEnrichDoc {
    state: Arc<Mutex<State>>,
}

impl VectorEnrichDoc {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for VectorEnrichDoc {
    fn name(&self) -> &str {
        "vector enrich-doc"
    }

    fn signature(&self) -> Signature {
        Signature::build("vector enrich-doc")
            .required(
                "field",
                SyntaxShape::String,
                "the field from which the vector is generated",
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
            .named(
                "id-column",
                SyntaxShape::String,
                "the name of the id column if used with an input stream",
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
        vector_enrich_doc(self.state.clone(), engine_state, stack, call, input)
    }
}

fn vector_enrich_doc(
    _state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;

    let field: String = call.req(engine_state, stack, 0)?;

    let mut field_contents: Vec<String> = vec![];
    let mut input_records: Vec<(Vec<String>, Vec<Value>)> = vec![];

    let max_tokens: usize = match call.get_flag::<usize>(engine_state, stack, "maxTokens")? {
        Some(t) => t,
        None => MAX_FREE_TIER_TOKENS,
    };

    let id_column: String = call
        .get_flag(engine_state, stack, "id-column")?
        .unwrap_or_else(|| "id".to_string());

    match input.into_value(span) {
        Value::List { vals, span: _span } => {
            // Have a list of records of the form:
            // cols = (bucket-name, database)
            // vals = (Record<doc contents>, string)
            for v in vals {
                let rec = match v.as_record() {
                    Ok(r) => r,
                    Err(e) => {
                        return Err(ShellError::GenericError(
                            "Could not parse input from query".to_string(),
                            "".to_string(),
                            None,
                            None,
                            vec![e],
                        ));
                    }
                };

                let doc_json = match rec.1[0].as_record() {
                    Ok(r) => r,
                    Err(e) => {
                        return Err(ShellError::GenericError(
                            "Could not parse input from query".to_string(),
                            "".to_string(),
                            None,
                            None,
                            vec![e],
                        ));
                    }
                };

                let index = match doc_json.0.iter().position(|r: &String| *r == field) {
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

                let content = match doc_json.1[index].as_string() {
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
                let new = (doc_json.0.to_vec(), doc_json.1.to_vec());
                if content != "" {
                    field_contents.push(content);
                    input_records.push(new);
                }
            }
        }
        _ => {
            return Err(ShellError::GenericError(
                "Piped input must be a list of json docs".to_string(),
                "".to_string(),
                None,
                None,
                Vec::new(),
            ));
        }
    };

    let key = match read_openai_api_key(engine_state) {
        Ok(k) => k,
        Err(e) => {
            return Err(e);
        }
    };

    let client =
        Client::with_config(async_openai::config::OpenAIConfig::default().with_api_key(key));

    let bpe = p50k_base().unwrap();
    let tokens = bpe.encode_with_special_tokens(&field_contents.join(" "));

    debug!("Total tokens: {:?}\n", tokens.len());

    //Regardless of token limit OpenAI's API can only accept arrays of strings up to 2048 in length
    let num_batches = (tokens.len() / max_tokens) + 1;
    let batch_size = cmp::min(2047, field_contents.len() / num_batches);

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

            if tokens.len() > max_tokens {
                upper = upper - batch_size / 2;
            }

            batches.push(field_contents[lower..=upper].to_vec());
            lower = upper + 1;
            upper += batch_size;

            if upper >= field_contents.len() {
                upper = field_contents.len() - 1;
            }
        }
    };

    let mut records = vec![];
    let start = SystemTime::now();
    let mut count = 0;
    for (i, batch) in batches.iter().enumerate() {
        let batch_start = SystemTime::now();
        info!("\rEmbedding batch {:?}/{} ", i + 1, batches.len());

        if log::log_enabled!(log::Level::Debug) {
            let bpe = p50k_base().unwrap();
            let tokens = bpe.encode_with_special_tokens(&batch.join(" "));
            debug!("- Tokens: {:?}", tokens.len());
        }

        let request = CreateEmbeddingRequestArgs::default()
            .model("text-embedding-3-small")
            .dimensions(128 as u32)
            .input(batch.clone())
            .build()
            .unwrap();

        let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();
        let ctrl_c_fut = CtrlcFuture::new(ctrl_c);
        let embd = client.embeddings();
        let rt = Runtime::new().unwrap();
        let embeddings = match rt.block_on(async {
            select! {
                result = embd.create(request)  => {
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

        for (i, _) in batch.iter().enumerate() {
            let mut res_keys: Vec<String> = input_records[count].0.to_vec();
            res_keys.push(format!("{}Vector", field));

            let mut res_vals: Vec<Value> = input_records[count].1.to_vec();
            res_vals.push(Value::List {
                span,
                vals: embeddings.data[i]
                    .embedding
                    .clone()
                    .iter()
                    .map(|&e| Value::Float {
                        val: e as f64,
                        span,
                    })
                    .collect(),
            });

            // TODO -handle this error
            let index = match res_keys.iter().position(|r: &String| *r == id_column) {
                Some(i) => i,
                None => {
                    return Err(ShellError::GenericError(
                            "Could not locate 'id' field in docs, if not called 'id' specify using --id-column".to_string(),
                            "".to_string(),
                            None,
                            None,
                            vec![],
                        ));
                }
            };
            let id = res_vals[index].clone();

            let vector_doc = Value::Record {
                cols: vec!["id".to_string(), "content".to_string()],
                vals: vec![
                    id,
                    Value::Record {
                        span,
                        cols: res_keys,
                        vals: res_vals,
                    },
                ],
                span,
            };
            records.push(vector_doc);

            count += 1;
        }

        let now = SystemTime::now();
        let difference = now.duration_since(batch_start);
        debug!("- Duration: {:?}", difference.unwrap());
    }
    let total_time = SystemTime::now().duration_since(start);
    debug!("\nTotal Duration: {:?}", total_time.unwrap());

    Ok(Value::List {
        span,
        vals: records,
    }
    .into_pipeline_data())
}
