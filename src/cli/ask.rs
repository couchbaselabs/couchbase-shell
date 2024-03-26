use crate::cli::llm_client::LLMClients;
use crate::cli::util::read_openai_api_key;
use crate::state::State;
use crate::OpenAIClient;
use nu_protocol::Example;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;
use tokio::select;

use crate::CtrlcFuture;
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
                "table of strings used as context for the question",
            )
            .category(Category::Custom("couchbase".to_string()))
    }

    fn usage(&self) -> &str {
        "Asks a connected LLM a question, optionally enhanced with context"
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Ask a simple question",
                example: "ask \"how do I use the ask command?\"",
                result: None,
            },
            Example {
                description: "Use the description field of 3 docs as context",
                example: "[airline_10 airline_11] | subdoc get description | select content | ask \"summarize this for me\"",
                result: None,
            },
        ]
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
    let mut context: Vec<String> = Vec::new();
    match input.into_value(span) {
        Value::List {
            vals,
            internal_span: span,
        } => {
            for v in vals {
                let rec = match v.as_record() {
                    Ok(r) => r,
                    Err(e) => {
                        return Err(ShellError::GenericError {
                            error: "Context must be a nushell table".to_string(),
                            msg: "".to_string(),
                            span: Some(span),
                            help: None,
                            inner: vec![e],
                        });
                    }
                };

                if rec.columns().len() > 1 {
                    return Err(ShellError::GenericError {
                        error: "Use 'select' to choose a single column to pipe to 'ask'"
                            .to_string(),
                        msg: "".to_string(),
                        span: Some(span),
                        help: None,
                        inner: vec![],
                    });
                }

                let ctx = rec.get_index(0).unwrap().1.as_str()?;
                context.push(ctx.to_string());
            }
        }
        _ => {}
    };

    let key = read_openai_api_key(state)?;
    let client = LLMClients::OpenAI(OpenAIClient::new(key, None));

    let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();
    let ctrl_c_fut = CtrlcFuture::new(ctrl_c);
    let rt = Runtime::new().unwrap();
    let answer = match rt.block_on(async {
        select! {
            answer = client.ask(question.clone(), context.clone()) => {
                match answer {
                    Ok(a) => Ok(a),
                    Err(e) => Err(e),
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
        Ok(a) => a,
        Err(e) => {
            return Err(e);
        }
    };

    Ok(Value::String {
        val: answer,
        internal_span: span,
    }
    .into_pipeline_data())
}
