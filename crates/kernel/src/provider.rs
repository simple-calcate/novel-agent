use crate::KernelError;
use async_trait::async_trait;
use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
use thiserror::Error;

/// 模型调用请求：内核层的稳定契约，由具体 Provider 翻译成自家 API。
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

/// 模型提供方抽象：内核只认流式输出，不关心 OpenAI/DeepSeek/Ollama 的协议细节。
#[async_trait]
pub trait ModelProvider: Send + Sync {
    fn name(&self) -> &str;

    async fn stream(
        &self,
        request: ModelRequest,
    ) -> Result<BoxStream<'static, Result<ModelChunk, ModelError>>, ModelError>;
}

/// 提供方配置：由宿主或扩展在运行期解析后交给注册表。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfig {
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub model: String,
}

/// Provider 工厂：注册表按名字创建实例，使模型接入完全可插拔。
pub trait ProviderFactory: Send + Sync {
    fn create(&self, config: &ProviderConfig) -> Result<Arc<dyn ModelProvider>, KernelError>;
}

#[derive(Default)]
pub struct ProviderRegistry {
    factories: BTreeMap<String, Arc<dyn ProviderFactory>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册（或覆盖）一个提供方工厂，返回被覆盖的工厂。
    pub fn register(
        &mut self,
        name: impl Into<String>,
        factory: impl ProviderFactory + 'static,
    ) -> Option<Arc<dyn ProviderFactory>> {
        self.factories.insert(name.into(), Arc::new(factory))
    }

    pub fn create(&self, config: &ProviderConfig) -> Result<Arc<dyn ModelProvider>, KernelError> {
        let factory = self
            .factories
            .get(&config.provider)
            .ok_or_else(|| KernelError::ProviderNotFound(config.provider.clone()))?;
        factory.create(config)
    }

    pub fn names(&self) -> Vec<&str> {
        self.factories.keys().map(String::as_str).collect()
    }
}

/// 在 Provider 未回报 usage 时，用字符数粗估输出 token（中文约 2 字符 = 1 token）。
pub fn estimate_output_tokens(text: &str) -> u32 {
    text.chars().count().div_ceil(2) as u32
}
