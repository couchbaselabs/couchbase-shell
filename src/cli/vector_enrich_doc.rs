use crate::state::State;
use log::debug;
use std::time::SystemTime;

use crate::cli::llm_client::LLMClients;
use crate::cli::util::read_openai_api_key;
use crate::CtrlcFuture;
use crate::OpenAIClient;
use nu_protocol::Example;
use std::convert::TryFrom;
use std::str;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;
use tokio::select;

use nu_engine::CallExt;

use nu_protocol::ast::Call;
use nu_protocol::engine::Command;
use nu_protocol::engine::{EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};

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

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Open local json doc and enrich the field named 'description'",
                example: "open ./local.json | vector enrich-doc",
                result: None,
            },
            Example {
                description:
                    "Fetch a single doc with id '12345' and enrich the field named 'description'",
                example: "doc get 12345 | select content | vector enrich-doc description",
                result: None,
            },
            Example {
                description: "Fetch and enrich all landmark documents from travel sample and upload the results to couchabase",
                example: "query  'SELECT * FROM `travel-sample` WHERE type = \"landmark\"' | select content | vector enrich-doc content | doc upsert",
                result: None,
            },
        ]
    }
}

fn vector_enrich_doc(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;

    let field: String = call.req(engine_state, stack, 0)?;

    let mut field_contents: Vec<String> = vec![];
    let mut input_records: Vec<(Vec<String>, Vec<Value>)> = vec![];

    let max_tokens: Option<usize> = call.get_flag::<usize>(engine_state, stack, "maxTokens")?;

    let id_column: String = call
        .get_flag(engine_state, stack, "id-column")?
        .unwrap_or_else(|| "id".to_string());

    let dim = match call.get_flag::<i64>(engine_state, stack, "dimension")? {
        Some(d) => u32::try_from(d).ok().unwrap(),
        None => 128,
    };

    match input.into_value(span) {
        Value::List { vals, span: _span } => {
            // This is able to parse a list of records, where the first value in each record is the contents
            // of a json document. This allows it to work with the output of query and 'doc commands | select content'
            for v in vals {
                // Read each record from the list of records
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

                // rec.1 are the values of the record, then the first value is the json of the document
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
                if content != "" {
                    field_contents.push(content);
                    input_records.push((doc_json.0.to_vec(), doc_json.1.to_vec()));
                }
            }
        }
        Value::Record {
            cols,
            vals,
            span: _,
        } => {
            let index = match cols.iter().position(|r: &String| *r == field) {
                Some(i) => i,
                None => {
                    return Err(ShellError::GenericError(
                        format!("The field '{}' must be present in input record", field),
                        "".to_string(),
                        None,
                        None,
                        Vec::new(),
                    ));
                }
            };

            let content = match vals[index].as_string() {
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

            field_contents.push(content);
            input_records.push((cols.to_vec(), vals.to_vec()));
        }
        _ => {
            return Err(ShellError::GenericError(
                "Piped input must a json doc or a list of json docs".to_string(),
                "".to_string(),
                None,
                None,
                Vec::new(),
            ));
        }
    };

    let key = read_openai_api_key(state)?;
    let client = LLMClients::OpenAI(OpenAIClient::new(key, max_tokens));

    let batches = client.batch_chunks(field_contents);

    let mut records = vec![];
    let start = SystemTime::now();
    let mut count = 0;
    for (i, batch) in batches.iter().enumerate() {
        let batch_start = SystemTime::now();
        println!("\rEmbedding batch {:?}/{} ", i + 1, batches.len());

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

        for (i, _) in batch.iter().enumerate() {
            let mut res_keys: Vec<String> = input_records[count].0.to_vec();
            res_keys.push(format!("{}Vector", field));

            let mut res_vals: Vec<Value> = input_records[count].1.to_vec();
            res_vals.push(Value::List {
                span,
                vals: embeddings[i]
                    .iter()
                    .map(|&e| Value::Float {
                        val: e as f64,
                        span,
                    })
                    .collect(),
            });

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

            // So that we can pipe the output of this straight into doc upsert the ouput needs to be records formatted like this
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
