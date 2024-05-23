use crate::client::bedrock_client::BedrockClient;
use crate::client::gemini_client::GeminiClient;
use crate::client::openai_client::OpenAIClient;
use crate::state::{Provider, State};
use async_trait::async_trait;
use nu_protocol::ShellError;
use std::sync::{Arc, Mutex};

#[async_trait]
pub trait LLMClient {
    fn batch_chunks(&self, chunks: Vec<String>) -> Vec<Vec<String>>;
    async fn embed(
        &self,
        batch: &Vec<String>,
        dim: Option<usize>,
    ) -> Result<Vec<Vec<f32>>, ShellError>;
    async fn ask(
        &self,
        question: String,
        context: Vec<String>,
    ) -> Result<nu_protocol::Value, ShellError>;
}

pub enum LLMClients {
    OpenAI(OpenAIClient),
    Gemini(GeminiClient),
    Bedrock(BedrockClient),
}

impl LLMClients {
    pub fn batch_chunks(&self, chunks: Vec<String>) -> Vec<Vec<String>> {
        match self {
            Self::OpenAI(c) => c.batch_chunks(chunks),
            Self::Gemini(c) => c.batch_chunks(chunks),
            Self::Bedrock(c) => c.batch_chunks(chunks),
        }
    }

    pub async fn embed(
        &self,
        batch: &Vec<String>,
        dim: Option<usize>,
    ) -> Result<Vec<Vec<f32>>, ShellError> {
        match self {
            Self::OpenAI(c) => c.embed(batch, dim).await,
            Self::Gemini(c) => c.embed(batch, dim).await,
            Self::Bedrock(c) => c.embed(batch, dim).await,
        }
    }

    pub async fn ask(&self, question: String, context: Vec<String>) -> Result<String, ShellError> {
        match self {
            Self::OpenAI(c) => c.ask(question, context).await,
            Self::Gemini(c) => c.ask(question, context).await,
            Self::Bedrock(c) => c.ask(question, context).await,
        }
    }

    pub fn new(
        state: Arc<Mutex<State>>,
        max_tokens: impl Into<Option<usize>>,
    ) -> Result<LLMClients, ShellError> {
        let guard = state.lock().unwrap();
        let (provider, api_key) = match guard.llm() {
            Some(llm) => (llm.provider(), llm.api_key()),
            None => {
                return Err(ShellError::GenericError {
                    error: "no llm config specified in config file".to_string(),
                    msg: "".to_string(),
                    span: None,
                    help: None,
                    inner: Vec::new(),
                });
            }
        };

        let client = match provider {
            Provider::OpenAI => LLMClients::OpenAI(OpenAIClient::new(api_key, max_tokens)?),
            Provider::Gemini => LLMClients::Gemini(GeminiClient::new(api_key, max_tokens)?),
            Provider::Bedrock => LLMClients::Bedrock(BedrockClient::new()),
        };

        Ok(client)
    }
}
