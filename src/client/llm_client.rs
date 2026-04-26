use crate::cli::no_llm_configured;
use crate::client::bedrock_client::BedrockClient;
use crate::client::gemini_client::GeminiClient;
use crate::client::openai_client::OpenAIClient;
use crate::state::{Provider, State};
use nu_protocol::ShellError;
use std::sync::{Arc, Mutex};

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
        model: String,
    ) -> Result<Vec<Vec<f32>>, ShellError> {
        match self {
            Self::OpenAI(c) => c.embed(batch, dim, model).await,
            Self::Gemini(c) => c.embed(batch, dim, model).await,
            Self::Bedrock(c) => c.embed(batch, dim, model).await,
        }
    }

    pub async fn ask(
        &self,
        question: String,
        template: Option<String>,
        context: Vec<String>,
        model: String,
    ) -> Result<String, ShellError> {
        match self {
            Self::OpenAI(c) => c.ask(question, template, context, model).await,
            Self::Gemini(c) => c.ask(question, template, context, model).await,
            Self::Bedrock(c) => c.ask(question, template, context, model).await,
        }
    }

    pub fn new(
        state: Arc<Mutex<State>>,
        max_tokens: impl Into<Option<usize>>,
    ) -> Result<LLMClients, ShellError> {
        let guard = state.lock().unwrap();
        let (provider, api_key, api_base) = match guard.active_llm() {
            Some(llm) => (llm.provider(), llm.api_key(), llm.api_base()),
            None => {
                return Err(no_llm_configured());
            }
        };

        let client = match provider {
            Provider::OpenAI => {
                LLMClients::OpenAI(OpenAIClient::new(api_key, max_tokens, api_base)?)
            }
            Provider::Gemini => {
                LLMClients::Gemini(GeminiClient::new(api_key, max_tokens, api_base)?)
            }
            Provider::Bedrock => LLMClients::Bedrock(BedrockClient::new(api_base)?),
        };

        Ok(client)
    }
}
