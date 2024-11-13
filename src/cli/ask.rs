use crate::client::{ClientError, LLMClients};
use crate::state::State;
use nu_protocol::{Example, IntoValue};
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;
use tokio::select;

use crate::cli::{client_error_to_shell_error, generic_error, no_llm_configured};
use crate::CtrlcFuture;
use nu_engine::command_prelude::Call;
use nu_engine::CallExt;
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
            .named(
                "model",
                SyntaxShape::String,
                "the chat model to ask the question",
                None,
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
                description: "Use the content field of 2 docs as context",
                example: "[landmark_10019 landmark_10020] | subdoc get content | select content | ask \"summarize this for me\"",
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
    let context: Vec<String> = match call.opt(engine_state, stack, 1)? {
        Some(ctx) => ctx,
        None => {
            match input.into_value(span)? {
                Value::List {
                    vals,
                    internal_span: span,
                } => {
                    let mut ctx: Vec<String> = Vec::new();
                    for v in vals {
                        let rec = match v.as_record() {
                            Ok(r) => r,
                            Err(_) => {
                                return Err(generic_error(
                                    "Piped context must be a nushell table",
                                    "Run 'ask --help' for an example".to_string(),
                                    span,
                                ));
                            }
                        };

                        if rec.columns().len() > 1 {
                            return Err(generic_error(
                            "Too many columns in context",
                            "Use 'select' to choose a single column. Run 'ask --help' for an example".to_string(),
                            span
                        ));
                        }

                        let ctx_string = match rec.get_index(0) {
                            Some(r) => match r.1.clone().into_value(span) {
                                Value::String { val, .. } => val,
                                _ => {
                                    return Err(generic_error(
                                    format!("context must be strings, {:?} supplied", r.1.get_type()),
                                    "Remove ask command from pipeline and check data being piped in".to_string(),
                                    span,
                                ));
                                }
                            },
                            None => {
                                return Err(generic_error(
                                    "question context is empty",
                                    "Remove ask command from pipeline and check data being piped in".to_string(),
                                    span));
                            }
                        };
                        ctx.push(ctx_string.to_string());
                    }
                    ctx
                }
                _ => {
                    vec![]
                }
            }
        }
    };

    let model = match call.get_flag::<String>(engine_state, stack, "model")? {
        Some(m) => m,
        None => {
            let guard = state.lock().unwrap();
            let model = match guard.active_llm() {
                Some(m) => {
                    match m.chat_model() {
                        Some(m) => m,
                        None => {
                            return Err(generic_error(
                                "no chat_model provided",
                                "supply the chat_model in the config file or using the --model flag".to_string(),
                                span));
                        }
                    }
                }
                None => {
                    return Err(no_llm_configured());
                }
            };
            model
        }
    };

    let client = LLMClients::new(state, None)?;

    let signals = engine_state.signals().clone();
    let signals_fut = CtrlcFuture::new(signals);
    let rt = Runtime::new().unwrap();
    let answer = match rt.block_on(async {
        select! {
            answer = client.ask(question.clone(), context.clone(), model) => {
                match answer {
                    Ok(a) => Ok(a),
                    Err(e) => Err(e),
                }
            },
            () = signals_fut =>
                Err(client_error_to_shell_error(ClientError::Cancelled{key: None}, span)),
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
