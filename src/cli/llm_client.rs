use async_openai::{types::CreateEmbeddingRequestArgs, Client};
use async_trait::async_trait;
use log::debug;
use nu_protocol::ShellError;
use tiktoken_rs::p50k_base;

#[async_trait]
pub trait LLMClient {
    fn batch_chunks(&self, chunks: Vec<String>) -> Vec<Vec<String>>;
    async fn embed(&self, batch: &Vec<String>, dim: u32) -> Result<Vec<Vec<f32>>, ShellError>;
}

pub enum LLMClients {
    OpenAI(OpenAIClient),
}

impl LLMClients {
    pub fn batch_chunks(&self, chunks: Vec<String>) -> Vec<Vec<String>> {
        match self {
            Self::OpenAI(c) => c.batch_chunks(chunks),
        }
    }

    pub async fn embed(&self, batch: &Vec<String>, dim: u32) -> Result<Vec<Vec<f32>>, ShellError> {
        match self {
            Self::OpenAI(c) => c.embed(batch, dim).await,
        }
    }
}
pub struct OpenAIClient {
    api_key: String,
    max_tokens: usize,
}

const OPENAI_MAX_FREE_TIER_TOKENS: usize = 150000;

impl OpenAIClient {
    pub fn new(api_key: String, max_tokens: Option<usize>) -> Self {
        let max_tokens = match max_tokens {
            Some(mt) => mt,
            None => OPENAI_MAX_FREE_TIER_TOKENS,
        };

        Self {
            api_key,
            max_tokens,
        }
    }

    fn batch_chunks(&self, chunks: Vec<String>) -> Vec<Vec<String>> {
        let bpe = p50k_base().unwrap();
        let tokens = bpe.encode_with_special_tokens(&chunks.join(" "));

        debug!("Total tokens: {:?}\n", tokens.len());

        let num_batches = (tokens.len() / self.max_tokens) + 1;
        let batch_size = chunks.len() / num_batches;
        let mut batches: Vec<Vec<String>> = Vec::new();
        if num_batches == 1 {
            batches.push(chunks.to_vec());
        } else {
            let mut lower = 0;
            let mut upper = batch_size;
            while lower < chunks.len() {
                batches.push(chunks[lower..=upper].to_vec());
                lower = upper + 1;
                upper += batch_size;

                if upper >= chunks.len() {
                    upper = chunks.len() - 1;
                }
            }
        };
        batches
    }

    async fn embed(&self, batch: &Vec<String>, dim: u32) -> Result<Vec<Vec<f32>>, ShellError> {
        let client = Client::with_config(
            async_openai::config::OpenAIConfig::default().with_api_key(self.api_key.clone()),
        );

        if log::log_enabled!(log::Level::Debug) {
            let bpe = p50k_base().unwrap();
            let tokens = bpe.encode_with_special_tokens(&batch.join(" "));
            debug!("- Tokens: {:?}", tokens.len());
        }

        let request = CreateEmbeddingRequestArgs::default()
            .model("text-embedding-3-small")
            .dimensions(dim)
            .input(batch.clone())
            .build()
            .unwrap();

        let embeddings = client.embeddings();
        let response = match embeddings.create(request).await {
            Ok(r) => r,
            Err(e) => {
                return Err(ShellError::GenericError(
                    format!("failed to execute request: {}", e),
                    "".to_string(),
                    None,
                    None,
                    Vec::new(),
                ))
            }
        };

        let mut rec: Vec<Vec<f32>> = vec![];
        for embd in response.data {
            rec.push(embd.embedding);
        }

        Ok(rec)
    }
}
