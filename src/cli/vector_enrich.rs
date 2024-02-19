use crate::cli::util::convert_nu_value_to_json_value;
use crate::state::State;

use async_openai::{types::CreateEmbeddingRequestArgs, Client};
use nu_command::Table;
use serde_json::Map;
use std::str;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;

use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::Command;
use nu_protocol::engine::{EngineState, Stack};
use nu_protocol::Value::Record;
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Type, Value,
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

    println!("INPUT");
    println!("{:?}", input.len());

    let mut source: Vec<String> = vec![];
    for i in &input {
        println!("{:?}", i);

        // Access second tuple of the record, the first is ["bucket_name", "database"]
        let record = i.as_record().unwrap().1[0].as_record().unwrap();
        println!("bucket record");
        println!("{:?}", record);

        //TESTING - echo the ID
        let index = record.0.iter().position(|r: &String| *r == "id").unwrap();
        println!("{:?}", field);
        println!("{:?}", record.1[index].as_string());
        println!("");

        // Find the index of the named field, handle the field missing here
        let index = record.0.iter().position(|r| *r == field).unwrap();
        println!("{:?}", field);
        println!("{:?}", record.1[index].as_string());
        println!("");

        source.push(record.1[index].as_string().unwrap())
    }

    // TO DO - batch requests if source is sufficiently long

    // For each string of text in source
    // Batch into chunks and get the embedding
    let rt = Runtime::new().unwrap();
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

    let request = CreateEmbeddingRequestArgs::default()
        .model("text-embedding-3-small")
        .dimensions(128 as u32)
        .input(source.clone())
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

    let mut count = 0;
    let mut records = vec![];
    for i in &input {
        let record = i.as_record().unwrap().1[0].as_record().unwrap();

        let mut res_string: Vec<String> = record.0.to_vec();
        res_string.push(format!("{}Vector", field));

        let mut res_vals: Vec<Value> = record.1.to_vec();
        res_vals.push(Value::String {
            span,
            val: format!("{:?}", response.data[count].embedding.clone()),
        });

        records.push(Value::Record {
            span,
            cols: res_string,
            vals: res_vals,
        });

        count += 1;
    }

    Ok(Value::List {
        span,
        vals: records,
    }
    .into_pipeline_data())
}
