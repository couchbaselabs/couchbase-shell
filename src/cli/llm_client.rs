use async_openai::{types::CreateEmbeddingRequestArgs, Client};
use async_trait::async_trait;
use log::{debug, info};
use nu_protocol::ShellError;
use ollama_rs::Ollama;
use tiktoken_rs::p50k_base;

#[async_trait]
pub trait LLMClient {
    fn batch_chunks(&self, chunks: Vec<String>) -> Vec<Vec<String>>;
    async fn embed(&self, batch: &Vec<String>, dim: u32) -> Result<Vec<Vec<f32>>, ShellError>;
}

pub enum LLMClients {
    OpenAI(OpenAIClient),
    Llama(LlamaClient),
}

impl LLMClients {
    pub fn batch_chunks(&self, chunks: Vec<String>) -> Vec<Vec<String>> {
        match self {
            Self::OpenAI(c) => c.batch_chunks(chunks),
            Self::Llama(c) => c.batch_chunks(chunks),
        }
    }

    pub async fn embed(&self, batch: &Vec<String>, dim: u32) -> Result<Vec<Vec<f32>>, ShellError> {
        match self {
            Self::OpenAI(c) => c.embed(batch, dim).await,
            Self::Llama(c) => c.embed(batch, dim).await,
        }
    }
}

pub struct OpenAIClient {
    max_tokens: usize,
    api_key: String,
}

impl OpenAIClient {
    pub fn new(max_tokens: usize, api_key: String) -> Self {
        Self {
            max_tokens,
            api_key,
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

        //TODO make this a mapping process as is done to convert to Value
        let mut rec: Vec<Vec<f32>> = vec![];
        for embd in response.data {
            rec.push(embd.embedding);
        }

        Ok(rec)
    }
}

pub struct LlamaClient {
    conn_str: String,
}

impl LlamaClient {
    pub fn new(conn_str: String) -> Self {
        Self { conn_str }
    }
    fn batch_chunks(&self, chunks: Vec<String>) -> Vec<Vec<String>> {
        // Ollama api does not allow batching, so we must emebd the chunks one by one
        vec![chunks]
    }

    async fn embed(&self, batch: &Vec<String>, dim: u32) -> Result<Vec<Vec<f32>>, ShellError> {
        // TODO connect to client using conn_str
        // By default it will connect to localhost:11434
        let model = "qwen:1.8b";
        if self.conn_str == "" {
            let ollama = Ollama::default();
        }
        let ollama = Ollama::default();
        let res = ollama.list_local_models().await.unwrap();
        println!("MODELS : {:?}", res);

        let mut rec: Vec<Vec<f32>> = vec![];
        for (i, prompt) in batch.iter().enumerate() {
            info!("Embedding batch {:?}/{} ", i + 1, batch.len());

            let res = ollama
                .generate_embeddings(model.to_string(), prompt.clone(), None)
                .await
                .unwrap();

            rec.push(res.embeddings.iter().map(|e| *e as f32).collect());
        }
        Ok(rec)
    }
}
