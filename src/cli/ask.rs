use crate::state::State;
use async_openai::types::ChatCompletionRequestMessage;
use async_openai::types::ChatCompletionRequestSystemMessageArgs;
use async_openai::types::ChatCompletionRequestUserMessageArgs;
use async_openai::types::CreateChatCompletionRequestArgs;
use async_openai::Client;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;

use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct Ask {
    state: Arc<Mutex<State>>,
}

impl Ask {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for Ask {
    fn name(&self) -> &str {
        "ask"
    }

    fn signature(&self) -> Signature {
        Signature::build("ask")
            .required("question", SyntaxShape::String, "the question to be asked")
            .optional(
                "context",
                SyntaxShape::Any,
                "list of strings to be used as context",
            )
            .category(Category::Custom("couchbase".to_string()))
    }

    fn usage(&self) -> &str {
        "Asks chat GPT the question proveided, optionally enhanced with context"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        ask(self.state.clone(), engine_state, stack, call, input)
    }
}

pub fn ask(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;

    let question: String = call.req(engine_state, stack, 0)?;
    let context: Vec<String> = match call.opt(engine_state, stack, 1)? {
        Some(v) => v,
        None => vec![],
    };

    let mut context: Vec<String> = Vec::new();
    match input.into_value(span) {
        Value::List { vals, span } => {
            for v in vals {
                let rec = match v.as_record() {
                    Ok(r) => r,
                    Err(e) => {
                        return Err(ShellError::GenericError(
                            // This is a lie, we need a record who's first column is a list of strings
                            "Supply a list of strings".to_string(),
                            "".to_string(),
                            None,
                            None,
                            vec![e],
                        ));
                    }
                };
                // TO DO - proper error handling herecb
                context.push(rec.1[0].as_string().unwrap());
            }
        }
        _ => {}
    };
    let mut messages: Vec<ChatCompletionRequestMessage> = vec![];

    messages.push(
        ChatCompletionRequestSystemMessageArgs::default()
            .content("You are a helpful assistant.")
            .build()
            .unwrap()
            .into(),
    );

    for ctx in context {
        messages.push(
            ChatCompletionRequestSystemMessageArgs::default()
                .content(ctx)
                .build()
                .unwrap()
                .into(),
        )
    }

    messages.push(
        ChatCompletionRequestUserMessageArgs::default()
            .content(question)
            .build()
            .unwrap()
            .into(),
    );

    println!("MESSAGES: {}", messages.len());

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

    let request = CreateChatCompletionRequestArgs::default()
        .max_tokens(512u16)
        .model("gpt-3.5-turbo")
        .messages(
            messages, // ChatCompletionRequestSystemMessageArgs::default()
                     //     .content("You are a helpful assistant.")
                     //     .build()
                     //     .unwrap()
                     //     .into(),
                     // TO DO, make these be populated by the conent passed in
                     // ChatCompletionRequestSystemMessageArgs::default()
                     //     .content("Jack Westwood loves to Tango")
                     //     .build()
                     //     .unwrap()
                     //     .into(),
                     // ChatCompletionRequestSystemMessageArgs::default()
                     //     .content("But Jack's favourite dance is East Coast Swing.")
                     //     .build()
                     //     .unwrap()
                     //     .into(),
                     // ChatCompletionRequestUserMessageArgs::default()
                     //     .content(question)
                     //     .build()
                     //     .unwrap()
                     //     .into(),
        )
        .build()
        .unwrap();

    let rt = Runtime::new().unwrap();
    let response = rt
        .block_on(async { client.chat().create(request).await })
        .unwrap();

    // println!("Response:");
    // println!("{:?}", response);
    println!(
        "{:?}",
        response.choices[0].message.content.as_ref().unwrap()
    );

    Ok(Value::Nothing { span }.into_pipeline_data())
}
