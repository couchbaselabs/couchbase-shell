use crate::cli::{api_base_unsupported, generic_error};
use aws_sdk_bedrockruntime::operation::invoke_model::InvokeModelError;
use aws_sdk_bedrockruntime::primitives::Blob;
use nu_protocol::ShellError;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::str;

pub struct BedrockClient {}

// The max number of tokens that can be generated in text response for Titan Express models
const MAX_RESPONSE_TOKENS: i32 = 8192;

impl BedrockClient {
    pub fn new(api_base: Option<String>) -> Result<Self, ShellError> {
        if api_base.is_some() {
            return Err(api_base_unsupported("Bedrock".into()));
        }

        Ok(Self {})
    }

    pub fn batch_chunks(&self, chunks: Vec<String>) -> Vec<Vec<String>> {
        // AWS Bedrock only support batch processing on data stored in S3 buckets so we have to process chunks one at a time
        let mut batches: Vec<Vec<String>> = Vec::new();
        for chunk in chunks {
            batches.push(vec![chunk])
        }
        batches
    }

    pub async fn embed(
        &self,
        batch: &Vec<String>,
        dim: Option<usize>,
        model: String,
    ) -> Result<Vec<Vec<f32>>, ShellError> {
        let config = aws_config::load_from_env().await;
        let client = aws_sdk_bedrockruntime::Client::new(&config);

        let mut rec: Vec<Vec<f32>> = vec![];

        for text in batch {
            let prompt = if let Some(dimension) = dim {
                json!({
                    "inputText": text.to_string(),
                    "dimensions": dimension,
                })
            } else {
                json!({
                    "inputText": text.to_string(),
                })
            };

            let result = match client
                .invoke_model()
                .model_id(model.clone())
                .content_type("application/json")
                .body(Blob::new(serde_json::to_string(&prompt).unwrap()))
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    match e {
                        aws_smithy_runtime_api::client::result::SdkError::DispatchFailure(_) => {
                            return Err(generic_error(
                                "Failed to dispatch Bedrock embedding request",
                                "Check AWS credentials are correctly configured".to_string(),
                                None,
                            ));
                        }
                        aws_smithy_runtime_api::client::result::SdkError::ServiceError(err) => {
                            let (err_msg, help) = match err.err() {
                                InvokeModelError::ResourceNotFoundException(inner_err) => {
                                    (inner_err.message.as_ref().unwrap().to_string(),
                                    Some("Supply the name of the model as you would in a Bedrock API request.".to_string()))
                                },
                                InvokeModelError::AccessDeniedException(inner_err) => {
                                    (inner_err.message.as_ref().unwrap().to_string(),
                                    Some("Have you been granted access to this model in the AWS web console?".to_string()))
                                },
                                InvokeModelError::ValidationException(inner_err) => {
                                    (inner_err.message.as_ref().unwrap().to_string(),
                                    Some("Supply the model name as required for the Bedrock API and check that it supports the chosen dimensionality.".to_string()))
                                },
                                _ => {
                                    (format!("unexpected error returned from Bedrock API: {:?}", err.err()), None)
                                }
                            };
                            return Err(generic_error(err_msg, help, None));
                        }
                        _ => {
                            return Err(generic_error(
                                format!("unexpected error returned from Bedrock API: {:?}", e),
                                None,
                                None,
                            ));
                        }
                    };
                }
            };

            let bytes = result.body().as_ref();
            let res: EmbeddingResponse = serde_json::from_slice(bytes).unwrap();
            rec.push(res.embedding);
        }

        Ok(rec)
    }

    pub async fn ask(
        &self,
        question: String,
        template: Option<String>,
        context: Vec<String>,
        model: String,
    ) -> Result<String, ShellError> {
        let config = aws_config::load_from_env().await;
        let client = aws_sdk_bedrockruntime::Client::new(&config);

        let tpl_value = template.unwrap_or( "Please answer this question: \\\"{}\\\". Using the following context: \\\"{}\\\"".to_string());
        let mut rendered_tpl = tpl_value.replacen("{}", &*question, 1);
        rendered_tpl = rendered_tpl.replacen("{}", &*context.join(" "), 1);
        let question_with_ctx = if !context.is_empty() {
            rendered_tpl
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
            .model_id(model)
            .content_type("application/json")
            .body(Blob::new(serde_json::to_string(&prompt).unwrap()))
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                return Err(generic_error(format!(
                    "error returned from Bedrock API: {:?}", e),
                    "Please supply AWS SDK config and credentials in ~/.aws/config and ~/.aws/credentials files".to_string(),
                    None
                ));
            }
        };

        let bytes = result.body().as_ref();
        let ans: AskResponse = serde_json::from_slice(bytes).unwrap();

        if ans.results.is_empty() {
            return Err(generic_error(
                "no answer contained in the response",
                None,
                None,
            ));
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
