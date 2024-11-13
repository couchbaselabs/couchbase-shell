use crate::state::State;
use log::debug;
use std::time::SystemTime;

use crate::client::{ClientError, LLMClients};
use crate::CtrlcFuture;
use nu_protocol::Record;
use nu_protocol::{Example, Span};
use nu_utils::SharedCow;
use std::str;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;
use tokio::select;

use nu_engine::CallExt;

use crate::cli::{client_error_to_shell_error, generic_error};
use nu_engine::command_prelude::Call;
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
                "model",
                SyntaxShape::String,
                "the model to generate the embeddings with",
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
            .named(
                "id-column",
                SyntaxShape::String,
                "the name of the id column if used with an input stream",
                None,
            )
            .named(
                "vectorField",
                SyntaxShape::String,
                "the name of the field into which the embedding is written".to_string(),
                None,
            )
            .category(Category::Custom("couchbase".to_string()))
    }

    fn description(&self) -> &str {
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
                example: "open ./local.json | vector enrich-doc description --model amazon.titan-embed-text-v2:0",
                result: None,
            },
            Example {
                description:
                    "Fetch a single doc with id '12345' and enrich the field named 'description'",
                example: "doc get 12345 | vector enrich-doc description --model models/text-embedding-004",
                result: None,
            },
            Example {
                description: "Fetch and enrich all landmark documents from travel sample and upload the results to couchabase",
                example: "query  'SELECT meta().id, * FROM `travel-sample` WHERE type = \"landmark\"' | vector enrich-doc content --model amazon.titan-embed-text-v1 | doc upsert",
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
    let mut input_ids: Vec<String> = vec![];

    let max_tokens: Option<usize> = call.get_flag::<usize>(engine_state, stack, "maxTokens")?;

    let vector_field =
        if let Some(vf) = call.get_flag::<String>(engine_state, stack, "vectorField")? {
            vf
        } else {
            format!("{}Vector", field.clone())
        };

    let model = match call.get_flag::<String>(engine_state, stack, "model")? {
        Some(m) => m,
        None => {
            let guard = state.lock().unwrap();
            guard.active_embed_model()?
        }
    };

    let id_column: String = call
        .get_flag(engine_state, stack, "id-column")?
        .unwrap_or_else(|| "id".to_string());

    let dim = call.get_flag::<usize>(engine_state, stack, "dimension")?;

    match input.into_value(span)? {
        Value::List { vals, .. } => {
            // This is able to parse a list of records, where the first value in each record is the contents
            // of a json document. This allows it to work with the output of query and 'doc commands | select content'
            for v in vals {
                // Read each record from the list of records
                let rec = match v.as_record() {
                    Ok(r) => r,
                    Err(_) => {
                        return Err(could_not_parse_input_error(span));
                    }
                };

                // Check if the input is from a doc get
                let (doc_json, id) = if rec.contains("id")
                    && rec.contains("content")
                    && rec.contains("cas")
                    && rec.contains("error")
                    && rec.contains("cluster")
                {
                    // error is either Nothing which will result in an empty string or an error string
                    // in either case the double unwrap here is safe
                    let err = rec.get("error").unwrap().as_str().unwrap();
                    if !err.is_empty() {
                        return Err(generic_error(
                            format!("error from doc get input: {}", err),
                            None,
                            None,
                        ));
                    }

                    (
                        //Safe to unwrap as we have validated the presence of these cols in the record
                        rec.get("content").unwrap().as_record()?,
                        rec.get("id").unwrap().as_str()?.to_string(),
                    )
                } else {
                    // Else piped input is from a query, which needs to contain 3 columns, one to be used as the ID, one holding the json doc and finally one with the cluster
                    if rec.len() != 3 {
                        return Err(generic_error(
                            "input incorrectly formatted",
                            "Run 'vector enrich-doc --help' for examples with input from 'doc get' and 'query'".to_string(),
                            None
                        ));
                    }

                    let id = read_id(rec, id_column.clone())?;

                    // No need to check this is set after loop, since we know there are 3 columns one will not be id or cluster
                    let mut content_column = "".to_string();
                    for column in rec.columns() {
                        if column != "cluster" && *column != id_column {
                            content_column = column.clone();
                        }
                    }

                    let res = match rec.get(content_column).unwrap().as_record() {
                        Ok(r) => Ok(r),
                        Err(_) => Err(could_not_parse_input_error(span)),
                    }?;
                    (res, id)
                };

                let content = read_from_field(doc_json, field.clone(), span)?;

                //The API will return an error on empty strings
                if !content.is_empty() {
                    field_contents.push(content);
                    input_records.push(doc_json.clone());
                    input_ids.push(id);
                }
            }
        }
        Value::Record { val, .. } => {
            let content = read_from_field(&val.clone().into_owned(), field.clone(), span)?;
            let id = read_id(&val, id_column)?;

            field_contents.push(content);
            input_records.push(val.into_owned());
            input_ids.push(id);
        }
        _ => {
            return Err(could_not_parse_input_error(span));
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

        let signals = engine_state.signals().clone();
        let signals_fut = CtrlcFuture::new(signals);
        let rt = Runtime::new().unwrap();
        let embeddings = rt.block_on(async {
            select! {
                result = client.embed(batch, dim, model.clone()) => {
                    result
                },
                () = signals_fut =>
               Err(client_error_to_shell_error(ClientError::Cancelled{key: None}, span)),
            }
        })?;

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

            let cols = vec!["id".to_string(), "content".to_string()];
            let vals = vec![
                Value::String {
                    val: input_ids[count].clone(),
                    internal_span: span,
                },
                Value::Record {
                    val: SharedCow::new(input_records[count].clone()),
                    internal_span: span,
                },
            ];

            let vector_doc = Value::Record {
                val: SharedCow::new(Record::from_raw_cols_vals(cols, vals, span, span).unwrap()),
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

fn read_from_field(doc: &Record, field: String, span: Span) -> Result<String, ShellError> {
    match doc.get(field.clone()) {
        Some(c) => match c.as_str() {
            Ok(s) => Ok(s.to_string()),
            Err(_) => Err(field_contents_not_string_error(field.clone(), span)),
        },
        None => Err(field_missing_error(field.clone(), span)),
    }
}

fn read_id(rec: &Record, id_column: String) -> Result<String, ShellError> {
    match rec.get(id_column.clone()) {
        Some(id) => match id {
            Value::String { val, .. } => Ok(val.clone()),
            Value::Int { val, .. } => Ok(val.to_string()),
            _ => {
                 Err(generic_error(
                    "Contents of 'id' column must be Int or String",
                    "A different column can be used as the id of the resulting docs with the '--id-column' flag".to_string(),
                    None
                ))
            }
        },
        None => {
             Err(generic_error(
                "No 'id' field in input",
                "An 'id' field is required to use as the IDs for the created docs, if not called 'id' specify using --id-column".to_string(),
                None
            ))
        }
    }
}

fn could_not_parse_input_error(span: Span) -> ShellError {
    generic_error(
        "Could not parse piped input",
        "Piped input must be a json doc, or a list of json docs. Run  'vector enrich-doc --help' for examples".to_string(),
        span
    )
}

fn field_missing_error(field: String, span: Span) -> ShellError {
    generic_error(
        format!("Field {} not found in input docs", field.clone()),
        "Remove 'vector enrich-doc', re-run pipeline and check docs contain the field".to_string(),
        span,
    )
}

fn field_contents_not_string_error(field: String, span: Span) -> ShellError {
    generic_error(
        "Could not convert field contents to string",
        format!("Does the field {} contain a string?", field),
        span,
    )
}
