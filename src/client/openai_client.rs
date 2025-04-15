use crate::cli::{generic_error, llm_api_key_missing};
use async_openai::config::OPENAI_API_BASE;
use async_openai::types::{
    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
    ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs,
};
use async_openai::{types::CreateEmbeddingRequestArgs, Client};
use log::debug;
use nu_protocol::ShellError;
use tiktoken_rs::p50k_base;

pub struct OpenAIClient {
    api_key: String,
    max_tokens: usize,
    api_base: String,
}

const MAX_FREE_TIER_TOKENS: usize = 150000;

impl OpenAIClient {
    pub fn new(
        api_key: Option<String>,
        max_tokens: impl Into<Option<usize>>,
        api_base: Option<String>,
    ) -> Result<Self, ShellError> {
        let max_tokens = max_tokens.into().unwrap_or(MAX_FREE_TIER_TOKENS);
        let api_base = api_base.unwrap_or(OPENAI_API_BASE.into());

        if let Some(api_key) = api_key {
            Ok(Self {
                api_key,
                max_tokens,
                api_base,
            })
        } else {
            Err(llm_api_key_missing("OpenAI".to_string()))
        }
    }

    pub fn batch_chunks(&self, chunks: Vec<String>) -> Vec<Vec<String>> {
        let bpe = p50k_base().unwrap();
        let tokens = bpe.encode_with_special_tokens(&chunks.join(" "));

        debug!("Total tokens: {:?}\n", tokens.len());

        //Regardless of token limit OpenAI's API can only accept batches up to 2048 in length
        let num_batches = (tokens.len() / self.max_tokens) + 1;
        let batch_size = if (chunks.len() / num_batches) > 2047 {
            println!("Batch size limited to 2047");
            2047
        } else {
            chunks.len() / num_batches
        };

        let mut batches: Vec<Vec<String>> = Vec::new();
        if num_batches == 1 {
            batches.push(chunks.to_vec());
        } else {
            let mut lower = 0;
            let mut upper = batch_size;
            while lower < chunks.len() {
                let bpe = p50k_base().unwrap();
                let tokens =
                    bpe.encode_with_special_tokens(&chunks[lower..=upper].to_vec().join(" "));

                if tokens.len() > self.max_tokens {
                    upper -= batch_size / 2;
                }

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

    pub async fn embed(
        &self,
        batch: &[String],
        dim: Option<usize>,
        model: String,
    ) -> Result<Vec<Vec<f32>>, ShellError> {
        let client = Client::with_config(
            async_openai::config::OpenAIConfig::default()
                .with_api_key(self.api_key.clone())
                .with_api_base(self.api_base.clone()),
        );

        if log::log_enabled!(log::Level::Debug) {
            let bpe = p50k_base().unwrap();
            let tokens = bpe.encode_with_special_tokens(&batch.join(" "));
            debug!("- Tokens: {:?}", tokens.len());
        }

        let request = if let Some(d) = dim {
            CreateEmbeddingRequestArgs::default()
                .model(model.to_string())
                .dimensions(d as u32)
                .input(batch.to_owned())
                .build()
                .unwrap()
        } else {
            CreateEmbeddingRequestArgs::default()
                .model(model.to_string())
                .input(batch.to_owned())
                .build()
                .unwrap()
        };

        let embeddings = client.embeddings();
        let response = match embeddings.create(request).await {
            Ok(r) => r,
            Err(e) => {
                let msg = match e {
                    async_openai::error::OpenAIError::ApiError(err) => err.message.to_string(),
                    _ => {
                        format!("failed to execute request: {:?}", e)
                    }
                };
                return Err(generic_error(msg, None, None));
            }
        };

        let mut rec: Vec<Vec<f32>> = vec![];
        for embd in response.data {
            rec.push(embd.embedding);
        }

        Ok(rec)
    }

    pub async fn ask(
        &self,
        question: String,
        context: Vec<String>,
        model: String,
    ) -> Result<String, ShellError> {
        let mut messages: Vec<ChatCompletionRequestMessage> = vec![];

        // Primes the model to respond appropriately
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

        let client = Client::with_config(
            async_openai::config::OpenAIConfig::default()
                .with_api_key(self.api_key.clone())
                .with_api_base(self.api_base.clone()),
        );

        let request = CreateChatCompletionRequestArgs::default()
            .max_tokens(512u16)
            .model(model)
            .messages(messages)
            .build()
            .unwrap();

        let response = client.chat().create(request).await;

        let answer = match response {
            Ok(r) => r.choices[0].message.content.as_ref().unwrap().to_string(),
            Err(e) => {
                return Err(generic_error(
                    format!("failed to execute request: {}", e),
                    None,
                    None,
                ));
            }
        };

        Ok(answer)
    }
}
