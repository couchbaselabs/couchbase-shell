use aws_sdk_bedrockruntime::primitives::Blob;
use nu_protocol::ShellError;
use serde::{Deserialize, Serialize};
use std::str;

pub struct BedrockClient {}

// Tokens processed per minute for titan-embed-text-v2
// const MAX_FREE_TIER_TOKENS: usize = 300000;

// The max number of tokens that can be generated in text response for Titan Express models
const MAX_RESPONSE_TOKENS: i32 = 8192;

impl BedrockClient {
    pub fn new() -> Self {
        Self {}
    }

    pub fn batch_chunks(&self, chunks: Vec<String>) -> Vec<Vec<String>> {
        // AWS Bedrock only support batch processing on data stored in S3 buckets so we have to process chunks one at a time
        vec![chunks]
    }

    pub async fn embed(
        &self,
        batch: &Vec<String>,
        dim: Option<usize>,
    ) -> Result<Vec<Vec<f32>>, ShellError> {
        let config = aws_config::load_from_env().await;
        let client = aws_sdk_bedrockruntime::Client::new(&config);

        let mut rec: Vec<Vec<f32>> = vec![];

        let dimension = match dim {
            Some(d) => {
                if d != 256 && d != 512 && d != 1024 {
                    return Err(ShellError::GenericError {
                        error: "Bedrock supports embedding dimensions of 256, 512 and 1024"
                            .to_string(),
                        msg: "".to_string(),
                        span: None,
                        help: None,
                        inner: vec![],
                    });
                }
                d
            }
            None => 256,
        };

        for text in batch {
            let prompt = EmbeddingPromptBody {
                input_text: text.to_string(),
                dimensions: dimension,
            };

            let result = match client
                .invoke_model()
                .model_id("amazon.titan-embed-text-v2:0")
                .content_type("application/json")
                .body(Blob::new(serde_json::to_string(&prompt).unwrap()))
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    return Err(ShellError::GenericError {
                        error: format!("error returned from Bedrock API: {:?}", e),
                        msg: "".to_string(),
                        span: None,
                        help: Some("Please supply AWS SDK config and credentials in ~/.aws/config and ~/.aws/credentials files".to_string()),
                        inner: Vec::new(),
                    })
                }
            };

            let bytes = result.body().as_ref();

            let res: EmbeddingResponse = serde_json::from_slice(&bytes).unwrap();
            rec.push(res.embedding);
        }

        Ok(rec)
    }

    pub async fn ask(&self, question: String, context: Vec<String>) -> Result<String, ShellError> {
        let config = aws_config::load_from_env().await;
        let client = aws_sdk_bedrockruntime::Client::new(&config);

        let question_with_ctx = if context.len() > 0 {
            format!(
                "Please answer this question: \\\"{}\\\". Using the following context: \\\"{}\\\"",
                question,
                context.join(" ")
            )
        } else {
            question
        };

        let prompt = AskPromptBody {
            input_text: question_with_ctx,
            text_generation_config: TextGenerationConfig {
                max_token_count: MAX_RESPONSE_TOKENS,
            },
        };

        let result = match client
            .invoke_model()
            .model_id("amazon.titan-text-express-v1")
            .content_type("application/json")
            .body(Blob::new(serde_json::to_string(&prompt).unwrap()))
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                return Err(ShellError::GenericError {
                    error: format!("error returned from Bedrock API: {:?}", e),
                    msg: "".to_string(),
                    span: None,
                    help: Some("Please supply AWS SDK config and credentials in ~/.aws/config and ~/.aws/credentials files".to_string()),
                    inner: Vec::new(),
                });
            }
        };

        let bytes = result.body().as_ref();

        let ans: AskResponse = serde_json::from_slice(&bytes).unwrap();

        if ans.results.len() < 1 {
            return Err(ShellError::GenericError {
                error: "no answer contained in the response".to_string(),
                msg: "".to_string(),
                span: None,
                help: None,
                inner: Vec::new(),
            });
        }

        let answer = if ans.results[0].completion_reason == "LENGTH" {
            format!(
                "{} \n\nAnswer truncated as it exceeded max token response limit of {:?}",
                ans.results[0].output_text, MAX_RESPONSE_TOKENS
            )
        } else {
            ans.results[0].output_text.clone()
        };

        Ok(answer.to_string())
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EmbeddingPromptBody {
    input_text: String,
    dimensions: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AskPromptBody {
    input_text: String,
    text_generation_config: TextGenerationConfig,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TextGenerationConfig {
    max_token_count: i32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AskResponse {
    results: Vec<AskResult>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AskResult {
    output_text: String,
    completion_reason: String,
}

#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    embedding: Vec<f32>,
}
