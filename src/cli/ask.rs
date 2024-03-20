use crate::cli::llm_client::LLMClients;
use crate::cli::util::read_openai_api_key;
use crate::state::State;
use crate::OpenAIClient;
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
        Value::List { vals, span: _ } => {
            for v in vals {
                let rec = match v.as_record() {
                    Ok(r) => r,
                    Err(e) => {
                        return Err(ShellError::GenericError(
                            "Supply a table of strings".to_string(),
                            "".to_string(),
                            None,
                            None,
                            vec![e],
                        ));
                    }
                };

                let ctx = rec.1[0].as_string()?;
                context.push(ctx);
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
                 Err(ShellError::GenericError(
               "Request cancelled".to_string(),
                "".to_string(),
                None,
                None,
                Vec::new(),
            )),
        }
    }) {
        Ok(a) => a,
        Err(e) => {
            return Err(e);
        }
    };

    Ok(Value::String { val: answer, span }.into_pipeline_data())
}
