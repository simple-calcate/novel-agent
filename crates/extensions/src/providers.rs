//! 模型提供方扩展：回声 Provider 与 OpenAI 兼容 Provider（DeepSeek、
//! Moonshot、Ollama 等）。

use async_trait::async_trait;
use futures::stream::BoxStream;
use futures::StreamExt;
use novel_kernel::{
    Extension, KernelBuilder, KernelError, ModelChunk, ModelError, ModelProvider, ModelRequest,
    ProviderConfig, ProviderFactory,
};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::{Arc, OnceLock};

/// 未配置真实模型时的回退实现：原样报告任务，便于联调。
pub struct EchoProvider;

#[async_trait]
impl ModelProvider for EchoProvider {
    fn name(&self) -> &str {
        "echo"
    }

    async fn stream(
        &self,
        request: ModelRequest,
    ) -> Result<BoxStream<'static, Result<ModelChunk, ModelError>>, ModelError> {
        let text = format!(
            "请配置真实模型提供方。已收到写作任务：{}",
            request.user_prompt
        );
        Ok(Box::pin(futures::stream::iter(vec![Ok(ModelChunk {
            text,
            input_tokens: Some(0),
            output_tokens: Some(0),
            done: true,
        })])))
    }
}

pub struct EchoFactory;

impl ProviderFactory for EchoFactory {
    fn create(&self, _config: &ProviderConfig) -> Result<Arc<dyn ModelProvider>, KernelError> {
        Ok(Arc::new(EchoProvider))
    }
}

/// OpenAI 兼容 API 客户端（支持 DeepSeek、Moonshot、Ollama 等）。
pub struct OpenAICompatibleProvider {
    pub base_url: String,
    pub api_key: String,
    pub provider_name: String,
}

pub struct OpenAICompatibleFactory;

impl ProviderFactory for OpenAICompatibleFactory {
    fn create(&self, config: &ProviderConfig) -> Result<Arc<dyn ModelProvider>, KernelError> {
        if config.api_key.is_empty() {
            return Err(KernelError::Provider(ModelError::Unavailable(
                "api key is empty".into(),
            )));
        }
        if config.base_url.is_empty() {
            return Err(KernelError::Provider(ModelError::Unavailable(
                "base url is empty".into(),
            )));
        }
        Ok(Arc::new(OpenAICompatibleProvider {
            base_url: config.base_url.clone(),
            api_key: config.api_key.clone(),
            provider_name: config.provider.clone(),
        }))
    }
}

fn shared_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(reqwest::Client::new)
}

#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
    temperature: f32,
    stream: bool,
    stream_options: StreamOptions,
}

#[derive(Debug, Serialize)]
struct StreamOptions {
    include_usage: bool,
}

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
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

#[derive(Debug, Deserialize)]
struct ChatCompletionChunk {
    #[serde(default)]
    choices: Vec<ChunkChoice>,
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct ChunkChoice {
    delta: ChunkDelta,
}

#[derive(Debug, Deserialize)]
struct ChunkDelta {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Usage {
    prompt_tokens: Option<u32>,
    completion_tokens: Option<u32>,
}

#[async_trait]
impl ModelProvider for OpenAICompatibleProvider {
    fn name(&self) -> &str {
        // 生命周期只到自身；调用方需要 String 时自行 to_owned
        match self.provider_name.as_str() {
            "deepseek" => "deepseek",
            "openai" => "openai",
            "anthropic" => "anthropic",
            "moonshot" => "moonshot",
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
            stream_options: StreamOptions {
                include_usage: true,
            },
        };

        let response = shared_client()
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

        let parser = SseParser::default();
        let pending = VecDeque::<String>::new();
        let stream = response.bytes_stream();
        let mapped = futures::stream::unfold(
            (parser, pending, stream),
            |(mut parser, mut pending, mut stream)| async move {
                loop {
                    if let Some(data) = pending.pop_front() {
                        match serde_json::from_str::<ChatCompletionChunk>(&data) {
                            Ok(chunk) => {
                                if let Some(chunk) = translate_chunk(chunk) {
                                    return Some((Ok(chunk), (parser, pending, stream)));
                                }
                                continue;
                            }
                            Err(error) => {
                                return Some((
                                    Err(ModelError::InvalidResponse(format!(
                                        "{error}; data={data}"
                                    ))),
                                    (parser, pending, stream),
                                ));
                            }
                        }
                    }
                    if parser.done {
                        return None;
                    }
                    match stream.next().await {
                        Some(Ok(bytes)) => {
                            for data in parser.feed(&bytes) {
                                pending.push_back(data);
                            }
                        }
                        Some(Err(e)) => {
                            return Some((
                                Err(ModelError::Unavailable(e.to_string())),
                                (parser, pending, stream),
                            ));
                        }
                        None => return None,
                    }
                }
            },
        );

        Ok(Box::pin(mapped))
    }
}

/// 把 OpenAI 兼容分片翻译为内核 ModelChunk。
/// 注意：`finish_reason` 帧不能作为终止信号——开启 include_usage 后，
/// usage 统计帧在其之后到达，提前终止会丢失 token 计数。
/// 终止由 usage 帧或 [DONE]/流结束决定。
fn translate_chunk(chunk: ChatCompletionChunk) -> Option<ModelChunk> {
    if let Some(choice) = chunk.choices.first() {
        if let Some(content) = &choice.delta.content {
            return Some(ModelChunk {
                text: content.clone(),
                input_tokens: None,
                output_tokens: None,
                done: false,
            });
        }
        // 无内容的 finish_reason/空 delta 帧：跳过，等待 usage 或流结束
        return None;
    }
    // 带 stream_options.include_usage 时，最后一个分片只有 usage。
    let usage = chunk.usage?;
    Some(ModelChunk {
        text: String::new(),
        input_tokens: usage.prompt_tokens,
        output_tokens: usage.completion_tokens,
        done: true,
    })
}

/// 增量 SSE 解析器：跨网络分块缓冲半行，一次网络分块中的多条 data 行
/// 都会被完整取出（旧实现两者都会丢数据）。
#[derive(Debug, Default)]
pub struct SseParser {
    buffer: Vec<u8>,
    done: bool,
}

impl SseParser {
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<String> {
        self.buffer.extend_from_slice(bytes);
        let mut payloads = Vec::new();
        while let Some(nl) = self.buffer.iter().position(|&b| b == b'\n') {
            let mut line: Vec<u8> = self.buffer.drain(..=nl).collect();
            line.pop();
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            let line = String::from_utf8_lossy(&line);
            if let Some(data) = line.strip_prefix("data:") {
                let data = data.strip_prefix(' ').unwrap_or(data);
                if data == "[DONE]" {
                    self.done = true;
                } else if !data.is_empty() {
                    payloads.push(data.to_owned());
                }
            }
        }
        payloads
    }
}

/// 依据配置解析注册表里的提供方名字：
/// 已知名字直接用；未知名字但有 api_key 视为自定义 OpenAI 兼容服务；
/// 无 api_key 回退到 echo。
pub fn resolve_provider_name(config: &ProviderConfig) -> String {
    if config.api_key.is_empty() {
        return "echo".into();
    }
    match config.provider.as_str() {
        "" | "custom" => "custom".into(),
        "deepseek" | "openai" | "anthropic" | "moonshot" | "ollama" => config.provider.clone(),
        other => {
            tracing::warn!(provider = other, "未知提供方，按自定义 OpenAI 兼容服务处理");
            "custom".to_string()
        }
    }
}

/// 模型提供方扩展。
pub struct ProvidersExtension;

impl ProvidersExtension {
    pub const KNOWN: &'static [&'static str] = &[
        "deepseek",
        "openai",
        "anthropic",
        "moonshot",
        "ollama",
        "custom",
        "echo",
    ];
}

impl Extension for ProvidersExtension {
    fn id(&self) -> &str {
        "builtin.providers"
    }

    fn setup(&self, builder: &mut KernelBuilder) -> Result<(), KernelError> {
        for name in Self::KNOWN {
            if *name == "echo" {
                builder.register_provider_factory(name, EchoFactory);
            } else {
                builder.register_provider_factory(name, OpenAICompatibleFactory);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sse_parser_handles_multi_line_and_split_chunks() {
        let mut parser = SseParser::default();
        // 一个网络分块包含两条 data 行
        let payloads = parser.feed(b"data: {\"a\":1}\ndata: {\"a\":2}\n\n");
        assert_eq!(payloads, vec![r#"{"a":1}"#, r#"{"a":2}"#]);

        // 一条 data 行被拆到两个网络分块
        let payloads = parser.feed(b"data: {\"b\"");
        assert!(payloads.is_empty());
        let payloads = parser.feed(b":1}\n");
        assert_eq!(payloads, vec![r#"{"b":1}"#]);
    }

    #[test]
    fn sse_parser_handles_crlf_and_done() {
        let mut parser = SseParser::default();
        let payloads = parser.feed(b"data: {\"c\":3}\r\ndata: [DONE]\r\n");
        assert_eq!(payloads, vec![r#"{"c":3}"#]);
        assert!(parser.done);
    }

    #[test]
    fn resolve_provider_name_falls_back() {
        assert_eq!(resolve_provider_name(&ProviderConfig::default()), "echo");
        assert_eq!(
            resolve_provider_name(&ProviderConfig {
                provider: "deepseek".into(),
                api_key: "k".into(),
                ..Default::default()
            }),
            "deepseek"
        );
        assert_eq!(
            resolve_provider_name(&ProviderConfig {
                provider: "who".into(),
                api_key: "k".into(),
                ..Default::default()
            }),
            "custom"
        );
    }

    #[test]
    fn translate_chunk_skips_finish_frame() {
        // finish_reason 帧不是终止信号：usage 帧在其之后到达
        let chunk: ChatCompletionChunk =
            serde_json::from_str(r#"{"choices":[{"delta":{},"finish_reason":"stop"}]}"#).unwrap();
        assert!(translate_chunk(chunk).is_none());
    }

    #[test]
    fn translate_chunk_maps_usage_only_frame() {
        let chunk = ChatCompletionChunk {
            choices: vec![],
            usage: Some(Usage {
                prompt_tokens: Some(11),
                completion_tokens: Some(22),
            }),
        };
        let mapped = translate_chunk(chunk).unwrap();
        assert_eq!(mapped.input_tokens, Some(11));
        assert_eq!(mapped.output_tokens, Some(22));
        assert!(mapped.done);
    }

    #[test]
    fn translate_chunk_maps_content() {
        let chunk: ChatCompletionChunk = serde_json::from_str(
            r#"{"choices":[{"delta":{"content":"雾"},"finish_reason":null}]}"#,
        )
        .unwrap();
        let mapped = translate_chunk(chunk).unwrap();
        assert_eq!(mapped.text, "雾");
        assert!(!mapped.done);
    }

    #[test]
    fn factory_rejects_missing_config() {
        let error = OpenAICompatibleFactory
            .create(&ProviderConfig {
                provider: "deepseek".into(),
                ..Default::default()
            })
            .err()
            .expect("应拒绝空 api key");
        assert!(error.to_string().contains("api key"));
    }
}
