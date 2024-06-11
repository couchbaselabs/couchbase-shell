use crate::state::State;
use log::debug;
use std::time::SystemTime;

use crate::client::LLMClients;
use crate::CtrlcFuture;
use nu_protocol::Example;
use nu_protocol::Record;
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
                "dimension of the resulting embeddings",
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
            .named(
                "vectorField",
                SyntaxShape::String,
                "the name of the field into which the embedding is written, defaults to fieldVector".to_string(),
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
                example: "open ./local.json | vector enrich-doc description",
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
    let mut input_records: Vec<nu_protocol::Record> = vec![];

    let max_tokens: Option<usize> = call.get_flag::<usize>(engine_state, stack, "maxTokens")?;

    let vector_field =
        if let Some(vf) = call.get_flag::<String>(engine_state, stack, "vectorField")? {
            vf
        } else {
            format!("{}Vector", field.clone())
        };

    let id_column: String = call
        .get_flag(engine_state, stack, "id-column")?
        .unwrap_or_else(|| "id".to_string());

    let dim = call.get_flag::<usize>(engine_state, stack, "dimension")?;

    match input.into_value(span) {
        Value::List { vals, .. } => {
            // This is able to parse a list of records, where the first value in each record is the contents
            // of a json document. This allows it to work with the output of query and 'doc commands | select content'
            for v in vals {
                // Read each record from the list of records
                let rec = match v.as_record() {
                    Ok(r) => r,
                    Err(e) => {
                        return Err(ShellError::GenericError {
                            error: "Could not parse input from query".to_string(),
                            msg: "".to_string(),
                            span: None,
                            help: None,
                            inner: vec![e],
                        });
                    }
                };

                let doc_json = match rec.get_index(0).unwrap().1.as_record() {
                    Ok(r) => r,
                    Err(e) => {
                        return Err(ShellError::GenericError {
                            error: "Could not parse input from query".to_string(),
                            msg: "".to_string(),
                            span: None,
                            help: None,
                            inner: vec![e],
                        });
                    }
                };

                let content = match doc_json.get(field.clone()) {
                    Some(c) => match c.as_str() {
                        Ok(s) => s.to_string(),
                        Err(e) => {
                            return Err(ShellError::GenericError {
                                error: "The field to embed must be a string".to_string(),
                                msg: "".to_string(),
                                span: None,
                                help: None,
                                inner: vec![e],
                            });
                        }
                    },
                    None => {
                        return Err(ShellError::GenericError {
                            error: format!("The field '{}' must be present in input record", field),
                            msg: "".to_string(),
                            span: None,
                            help: None,
                            inner: vec![],
                        });
                    }
                };

                //The API will return an error on empty strings
                if content != "" {
                    field_contents.push(content);
                    input_records.push(doc_json.clone());
                }
            }
        }
        Value::Record { val, .. } => {
            let content = match val.get(field.clone()) {
                Some(c) => match c.as_str() {
                    Ok(s) => s.to_string(),
                    Err(e) => {
                        return Err(ShellError::GenericError {
                            error: "The field to embed must be a string".to_string(),
                            msg: "".to_string(),
                            span: None,
                            help: None,
                            inner: vec![e],
                        });
                    }
                },
                None => {
                    return Err(ShellError::GenericError {
                        error: format!("The field '{}' must be present in input record", field),
                        msg: "".to_string(),
                        span: None,
                        help: None,
                        inner: Vec::new(),
                    });
                }
            };

            field_contents.push(content);
            input_records.push(*val);
        }
        _ => {
            return Err(ShellError::GenericError {
                error: "Piped input must a json doc or a list of json docs".to_string(),
                msg: "".to_string(),
                span: None,
                help: None,
                inner: Vec::new(),
            });
        }
    };

    let client = LLMClients::new(state, max_tokens)?;

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
                        Err(e) => Err(ShellError::GenericError{
                        error: format!("failed to execute request: {}", e),
                        msg: "".to_string(),
                        span: None,
                        help: None,
                        inner: Vec::new(),
                    })
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

        for (i, _) in batch.iter().enumerate() {
            input_records[count].insert(
                vector_field.clone(),
                Value::List {
                    internal_span: span,
                    vals: embeddings[i]
                        .iter()
                        .map(|&e| Value::Float {
                            val: e as f64,
                            internal_span: span,
                        })
                        .collect(),
                },
            );

            let id = match input_records[count].get(id_column.clone()) {
                Some(id) => match id {
                    Value::String { val, .. } => val.clone(),
                    Value::Int { val, .. } => val.to_string(),
                    _ => {
                        return Err(ShellError::GenericError {
                            error: "Contents of ID columns must be Int or String".to_string(),
                            msg: "".to_string(),
                            span: None,
                            help: None,
                            inner: vec![],
                        });
                    }
                },
                None => {
                    return Err(ShellError::GenericError{
                            error: "Could not locate 'id' field in docs, if not called 'id' specify using --id-column".to_string(),
                            msg: "".to_string(),
                            span: None,
                            help: None,
                            inner: vec![],
                    });
                }
            };

            let cols = vec!["id".to_string(), "content".to_string()];
            let vals = vec![
                Value::String {
                    val: id,
                    internal_span: span,
                },
                Value::Record {
                    val: Box::new(input_records[count].clone()),
                    internal_span: span,
                },
            ];

            let vector_doc = Value::Record {
                val: Box::new(Record::from_raw_cols_vals(cols, vals, span, span).unwrap()),
                internal_span: span,
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
        internal_span: span,
        vals: records,
    }
    .into_pipeline_data())
}
