use crate::cli::{api_base_unsupported, generic_error, llm_api_key_missing};
use bytes::Bytes;
use log::info;
use nu_protocol::ShellError;
use reqwest::Response;
use serde::{Deserialize, Serialize};
use serde_json::{json, Error};
use tokio::{select, time::sleep, time::Duration};

pub struct GeminiClient {
    api_key: String,
    max_tokens: usize,
}

// While Gemini does not have a per request limit, the per minute token limit is used here
const MAX_FREE_TIER_TOKENS: usize = 1000000;

// According to the Gemini API docs: A token is equivalent to about 4 characters for Gemini models
const CHARS_PER_TOKEN: usize = 4;

// At most 100 requests can be in one batch
const MAX_BATCH_SIZE: usize = 100;

impl GeminiClient {
    pub fn new(
        api_key: Option<String>,
        max_tokens: impl Into<Option<usize>>,
        api_base: Option<String>,
    ) -> Result<Self, ShellError> {
        if api_base.is_some() {
            return Err(api_base_unsupported("Gemini".into()));
        }

        let max_tokens = max_tokens.into().unwrap_or(MAX_FREE_TIER_TOKENS);

        if let Some(api_key) = api_key {
            Ok(Self {
                api_key,
                max_tokens,
            })
        } else {
            Err(llm_api_key_missing("Gemini".to_string()))
        }
    }

    pub fn batch_chunks(&self, chunks: Vec<String>) -> Vec<Vec<String>> {
        let mut tokens = 0;
        let mut batch = vec![];
        let mut batches = vec![];
        for chunk in chunks {
            tokens += chunk.chars().count() / CHARS_PER_TOKEN;

            if tokens >= self.max_tokens || batch.len() == MAX_BATCH_SIZE {
                batches.push(batch);
                batch = vec![chunk.clone()];
                tokens = chunk.chars().count();
            } else {
                batch.push(chunk.to_string());
            }
        }

        batches.push(batch);
        batches
    }

    pub async fn embed(
        &self,
        batch: &Vec<String>,
        dim: Option<usize>,
        model: String,
    ) -> Result<Vec<Vec<f32>>, ShellError> {
        let model = if !model.contains("models/") {
            info!(
                "Gemini models must begin 'models/' so model name has been changed to 'models/{}'",
                model
            );
            format!("models/{}", model)
        } else {
            model
        };

        let url = format!(
            "https://generativelanguage.googleapis.com/v1/{}:batchEmbedContents?key={}",
            model, self.api_key
        );

        let mut batch_json = EmbeddingBatchRequest {
            requests: Vec::new(),
        };
        for str in batch {
            let mut request = json!(
                {
                    "model": model,
                    "content": {
                        "parts": [
                            {"text": str.to_string()}
                        ]
                    }
                }
            );

            if let Some(d) = dim {
                if d == 0 {
                    return Err(generic_error(
                        "Invalid embedding dimension",
                        "The dimension for embeddings must be greater than zero.".to_string(),
                        None,
                    ));
                }
                request["outputDimensionality"] = d.into()
            };
            batch_json.requests.push(request);
        }

        let res = execute_request(url, batch_json).await?;

        let bytes = read_response(res).await?;

        let embd: EmbeddingResponse = match serde_json::from_slice(&bytes) {
            Ok(e) => e,
            Err(e) => {
                return Err(failed_to_parse_response_error(e));
            }
        };

        let mut rec: Vec<Vec<f32>> = vec![];
        for vals in embd.embeddings {
            rec.push(vals.values);
        }

        Ok(rec)
    }

    pub async fn ask(
        &self,
        question: String,
        context: Vec<String>,
        model: String,
    ) -> Result<String, ShellError> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            model, self.api_key
        );

        let question_with_ctx = if !context.is_empty() {
            format!(
                "Please answer this question: \\\"{}\\\". Using the following context: \\\"{}\\\"",
                question,
                context.join(" ")
            )
        } else {
            question
        };

        let ask_request: AskRequest = AskRequest {
            contents: vec![Parts {
                parts: vec![Text {
                    text: question_with_ctx.clone(),
                }],
            }],
        };

        let res = execute_request(url, ask_request).await?;

        let bytes = read_response(res).await?;

        let ans: AskResponse = match serde_json::from_slice(&bytes) {
            Ok(a) => a,
            Err(e) => {
                return Err(failed_to_parse_response_error(e));
            }
        };

        Ok(ans.candidates[0].content.parts[0].text.clone())
    }
}

fn error_message(bytes: bytes::Bytes) -> String {
    #[derive(Deserialize, Debug)]
    struct ErrorResponse {
        error: Error,
    }

    #[derive(Deserialize, Debug)]
    struct Error {
        message: String,
    }

    let err_msg: ErrorResponse = serde_json::from_slice(&bytes).unwrap();

    err_msg.error.message
}

async fn read_response(res: Response) -> Result<Bytes, ShellError> {
    let status = res.status().as_u16();
    let bytes = match res.bytes().await {
        Ok(b) => b,
        Err(e) => {
            return Err(generic_error(
                format!("could not read response body: {}", e),
                None,
                None,
            ));
        }
    };

    if status != 200 {
        return Err(generic_error(error_message(bytes), None, None));
    };

    Ok(bytes)
}

async fn execute_request<T>(url: String, json_body: T) -> Result<Response, ShellError>
where
    T: Serialize,
{
    let client = reqwest::Client::new();

    let body = match serde_json::to_string(&json_body) {
        Ok(b) => b,
        Err(e) => {
            return Err(generic_error(
                format!("Could not create embedding request: {}", e),
                None,
                None,
            ));
        }
    };

    let res = match select! {
    res = client.post(url.clone()).body(body).send() => {
        match res {
            Ok(r) => Ok(r),
            Err(e) =>
                Err(generic_error(format!("Could not post ask request: {}", e), None, None))
            }
        },
        () =  sleep(Duration::from_secs(30)) =>
            Err(generic_error("Ask request timed out", None, None)),
    } {
        Ok(r) => r,
        Err(e) => return Err(e),
    };

    Ok(res)
}

#[derive(Serialize, Debug)]
struct EmbeddingBatchRequest {
    requests: Vec<serde_json::Value>,
}

#[derive(Serialize, Debug)]
struct Parts {
    parts: Vec<Text>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Text {
    text: String,
}

#[derive(Deserialize, Debug)]
struct EmbeddingResponse {
    embeddings: Vec<Values>,
}

#[derive(Deserialize, Debug)]
struct Values {
    values: Vec<f32>,
}

#[derive(Serialize, Debug)]
struct AskRequest {
    contents: Vec<Parts>,
}

#[derive(Deserialize, Debug)]
struct AskResponse {
    candidates: Vec<Candidate>,
}

#[derive(Deserialize, Debug)]
struct Candidate {
    content: Content,
}

#[derive(Deserialize, Debug)]
struct Content {
    parts: Vec<Text>,
    // role: String,
}

fn failed_to_parse_response_error(e: Error) -> ShellError {
    generic_error(
        format!("could not parse Gemini response: {}", e),
        None,
        None,
    )
}
