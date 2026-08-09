use async_trait::async_trait;
use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelRequest {
    pub model: String,
    pub system_prompt: String,
    pub user_prompt: String,
    pub max_tokens: u32,
    pub temperature: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelChunk {
    pub text: String,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
    pub done: bool,
}

#[derive(Debug, Error)]
pub enum ModelError {
    #[error("provider unavailable: {0}")]
    Unavailable(String),
    #[error("invalid response: {0}")]
    InvalidResponse(String),
    #[error("http error: {0}")]
    Http(String),
    #[error("budget exceeded")]
    BudgetExceeded,
}

#[async_trait]
pub trait ModelProvider: Send + Sync {
    fn name(&self) -> &'static str;
    async fn stream(
        &self,
        request: ModelRequest,
    ) -> Result<BoxStream<'static, Result<ModelChunk, ModelError>>, ModelError>;
}

pub struct EchoProvider;

#[async_trait]
impl ModelProvider for EchoProvider {
    fn name(&self) -> &'static str {
        "echo"
    }

    async fn stream(
        &self,
        request: ModelRequest,
    ) -> Result<BoxStream<'static, Result<ModelChunk, ModelError>>, ModelError> {
        let text = format!("请配置真实模型提供方。已收到写作任务：{}", request.user_prompt);
        Ok(Box::pin(futures::stream::iter(vec![Ok(ModelChunk {
            text,
            input_tokens: Some(0),
            output_tokens: Some(0),
            done: true,
        })])))
    }
}

/// OpenAI-compatible API 客户端（支持 DeepSeek、Moonshot 等）
pub struct OpenAICompatibleProvider {
    pub base_url: String,
    pub api_key: String,
    pub provider_name: String,
}

#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
    temperature: f32,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct ErrorResponse {
    error: Option<ErrorDetail>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ErrorDetail {
    message: Option<String>,
    code: Option<String>,
}

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionChunk {
    choices: Vec<ChunkChoice>,
}

#[derive(Debug, Deserialize)]
struct ChunkChoice {
    delta: ChunkDelta,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChunkDelta {
    content: Option<String>,
}

#[async_trait]
impl ModelProvider for OpenAICompatibleProvider {
    fn name(&self) -> &'static str {
        // 需要返回 &'static str，所以用 leak 或静态映射
        match self.provider_name.as_str() {
            "deepseek" => "deepseek",
            "openai" => "openai",
            "anthropic" => "anthropic",
            "ollama" => "ollama",
            _ => "custom",
        }
    }

    async fn stream(
        &self,
        request: ModelRequest,
    ) -> Result<BoxStream<'static, Result<ModelChunk, ModelError>>, ModelError> {
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let body = ChatCompletionRequest {
            model: request.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".into(),
                    content: request.system_prompt,
                },
                ChatMessage {
                    role: "user".into(),
                    content: request.user_prompt,
                },
            ],
            max_tokens: request.max_tokens,
            temperature: request.temperature,
            stream: true,
        };

        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ModelError::Unavailable(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            let message = serde_json::from_str::<ErrorResponse>(&text)
                .ok()
                .and_then(|r| r.error)
                .and_then(|e| e.message)
                .unwrap_or_else(|| text.clone());
            return Err(ModelError::Http(format!("{status}: {message}")));
        }

        let stream = response.bytes_stream();
        let mapped = futures::stream::unfold(stream, |mut stream| async move {
            use futures::StreamExt;
            loop {
                match stream.next().await {
                    Some(Ok(bytes)) => {
                        let text = String::from_utf8_lossy(&bytes);
                        for line in text.lines() {
                            if let Some(data) = line.strip_prefix("data: ") {
                                if data == "[DONE]" {
                                    return Some((Ok(ModelChunk {
                                        text: String::new(),
                                        input_tokens: None,
                                        output_tokens: None,
                                        done: true,
                                    }), stream));
                                }
                                if let Ok(chunk) = serde_json::from_str::<ChatCompletionChunk>(data) {
                                    if let Some(choice) = chunk.choices.first() {
                                        if let Some(content) = &choice.delta.content {
                                            return Some((Ok(ModelChunk {
                                                text: content.clone(),
                                                input_tokens: None,
                                                output_tokens: None,
                                                done: choice.finish_reason.is_some(),
                                            }), stream));
                                        }
                                        if choice.finish_reason.is_some() {
                                            return Some((Ok(ModelChunk {
                                                text: String::new(),
                                                input_tokens: None,
                                                output_tokens: None,
                                                done: true,
                                            }), stream));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Some(Err(e)) => {
                        return Some((Err(ModelError::Unavailable(e.to_string())), stream));
                    }
                    None => return None,
                }
            }
        });

        Ok(Box::pin(mapped))
    }
}
