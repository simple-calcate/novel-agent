//! 墨枢内核：最小、稳定的 Agent 执行核心。
//!
//! 内核只做四件事——组装模型请求、按预算消费输出流、分发工具调用、
//! 派发领域事件。模型提供方、工具、事件订阅者、持久化设施全部由
//! 扩展在组装期注册进来（见 `novel-extensions` 与 ADR 0007）。
//! 跨层接口总表：仓库根目录 `docs/interfaces.md`。

pub mod agent;
pub mod budget;
pub mod events;
pub mod provider;
pub mod services;
pub mod tool;

pub use agent::{AgentBudget, AgentSpec, ContinuationReport};
pub use budget::BudgetGuard;
pub use events::{DispatchSummary, EventBus, EventSubscriber, SubscriberOutcome};
pub use provider::{
    estimate_output_tokens, ModelChunk, ModelError, ModelProvider, ModelRequest, ProviderConfig,
    ProviderFactory, ProviderRegistry,
};
pub use services::Services;
pub use tool::{Tool, ToolContext, ToolDescriptor, ToolRegistry};

use novel_domain::{DomainEvent, EventId};
use serde_json::{json, Value};
use std::any::Any;
use std::sync::Arc;
use std::time::Instant;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum KernelError {
    #[error("provider not registered: {0}")]
    ProviderNotFound(String),
    #[error(transparent)]
    Provider(#[from] ModelError),
    #[error("tool not found: {0}")]
    ToolNotFound(String),
    #[error("tool {tool} failed: {message}")]
    ToolFailed { tool: String, message: String },
    #[error("service not registered: {0}")]
    ServiceNotFound(&'static str),
    #[error("extension {0} setup failed: {1}")]
    Extension(String, String),
    #[error("storage error: {0}")]
    Storage(String),
    #[error("budget exceeded: {0}")]
    BudgetExceeded(String),
}

/// 扩展：向内核注册提供方、工具、订阅者与服务。
/// 实现者可以在 `setup` 里注册任意数量的能力。
pub trait Extension: Send + Sync {
    fn id(&self) -> &str;

    fn setup(&self, builder: &mut KernelBuilder) -> Result<(), KernelError>;
}

#[derive(Default)]
pub struct KernelBuilder {
    providers: ProviderRegistry,
    tools: ToolRegistry,
    subscribers: Vec<Arc<dyn EventSubscriber>>,
    services: Services,
}

impl KernelBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// 注入一个服务（如 SQLite 仓库），扩展按类型取用。
    pub fn service<T>(mut self, service: Arc<T>) -> Self
    where
        T: Any + Send + Sync,
    {
        self.register_service(service);
        self
    }

    pub fn register_service<T>(&mut self, service: Arc<T>)
    where
        T: Any + Send + Sync,
    {
        self.services.insert(service);
    }

    pub fn provider_factory<F>(mut self, name: &str, factory: F) -> Self
    where
        F: ProviderFactory + 'static,
    {
        self.register_provider_factory(name, factory);
        self
    }

    pub fn register_provider_factory<F>(&mut self, name: &str, factory: F)
    where
        F: ProviderFactory + 'static,
    {
        self.providers.register(name, factory);
    }

    pub fn tool(mut self, tool: impl Tool + 'static) -> Self {
        self.register_tool(tool);
        self
    }

    pub fn register_tool(&mut self, tool: impl Tool + 'static) {
        self.tools.register(tool);
    }

    pub fn subscriber(mut self, subscriber: impl EventSubscriber + 'static) -> Self {
        self.add_subscriber(subscriber);
        self
    }

    pub fn add_subscriber(&mut self, subscriber: impl EventSubscriber + 'static) {
        self.subscribers.push(Arc::new(subscriber));
    }

    pub fn extension(mut self, extension: impl Extension + 'static) -> Result<Self, KernelError> {
        self.register_extension(extension)?;
        Ok(self)
    }

    /// 嵌套注册扩展（扩展内部组装多个子扩展时使用）。
    pub fn register_extension(
        &mut self,
        extension: impl Extension + 'static,
    ) -> Result<(), KernelError> {
        extension
            .setup(self)
            .map_err(|error| KernelError::Extension(extension.id().to_owned(), error.to_string()))
    }

    pub fn build(self) -> Result<Kernel, KernelError> {
        Ok(Kernel {
            providers: self.providers,
            tools: self.tools,
            events: EventBus::with_subscribers(self.subscribers),
            services: self.services,
        })
    }
}

pub struct Kernel {
    providers: ProviderRegistry,
    tools: ToolRegistry,
    events: EventBus,
    services: Services,
}

impl Kernel {
    pub fn builder() -> KernelBuilder {
        KernelBuilder::new()
    }

    /// 按名字执行工具，未注册的工具返回 `ToolNotFound`。
    pub async fn call_tool(&self, id: &str, input: Value) -> Result<Value, KernelError> {
        let tool = self
            .tools
            .get(id)
            .ok_or_else(|| KernelError::ToolNotFound(id.to_owned()))?;
        tool.execute(input, &ToolContext::new(self))
            .await
            .map_err(|error| match error {
                KernelError::ToolFailed { .. } | KernelError::ToolNotFound(_) => error,
                other => KernelError::ToolFailed {
                    tool: id.to_owned(),
                    message: other.to_string(),
                },
            })
    }

    /// 派发领域事件到所有匹配的订阅者。单个订阅者失败不影响其他订阅者。
    pub fn dispatch(&self, event: &DomainEvent) -> DispatchSummary {
        self.events.dispatch(self, event)
    }

    /// 创建 Provider 实例（每次调用新建，便于配置热更新）。
    pub fn provider(&self, config: &ProviderConfig) -> Result<Arc<dyn ModelProvider>, KernelError> {
        self.providers.create(config)
    }

    pub fn service<T>(&self) -> Result<Arc<T>, KernelError>
    where
        T: Any + Send + Sync,
    {
        self.services
            .get::<T>()
            .ok_or(KernelError::ServiceNotFound(std::any::type_name::<T>()))
    }

    pub fn tool_registry(&self) -> &ToolRegistry {
        &self.tools
    }

    pub fn provider_registry(&self) -> &ProviderRegistry {
        &self.providers
    }

    /// 执行一次续写：组装请求 → 流式消费（预算硬约束）→ 产出补丁，
    /// 可选发布 `agent.finished` 事件。
    pub async fn run_continuation(
        &self,
        config: &ProviderConfig,
        spec: AgentSpec,
    ) -> Result<ContinuationReport, KernelError> {
        let provider = self.provider(config)?;
        let started = Instant::now();
        let request = agent::build_request(&config.model, &spec);
        let stream = provider.stream(request).await?;
        let guard = BudgetGuard::new(&spec.budget);
        let outcome = agent::consume_stream(stream, guard).await?;
        let patch = agent::build_patch(&spec, outcome.text, provider.name());

        let report = ContinuationReport {
            truncated: outcome.truncated,
            input_tokens: outcome.input_tokens,
            output_tokens: outcome.output_tokens,
            elapsed_ms: started.elapsed().as_millis(),
            patch,
        };

        if spec.emit_finish_event {
            self.dispatch(&finish_event(&spec, &report));
        }

        Ok(report)
    }
}

fn finish_event(spec: &AgentSpec, report: &ContinuationReport) -> DomainEvent {
    DomainEvent {
        event_id: EventId::new(),
        event_type: "agent.finished".into(),
        schema_version: novel_domain::EVENT_SCHEMA_VERSION,
        occurred_at: chrono::Utc::now(),
        project_id: spec.project_id.clone(),
        book_id: None,
        chapter_id: Some(spec.chapter_id.clone()),
        scene_id: None,
        block_id: None,
        actor: novel_domain::Actor::Agent {
            model: "kernel".into(),
        },
        source: novel_domain::EventSource::Agent,
        platform: novel_domain::Platform::Unknown,
        transaction_id: uuid::Uuid::new_v4().to_string(),
        correlation_id: None,
        causation_id: Some(spec.id.to_string()),
        revision_before: spec.base_revision,
        revision_after: spec.base_revision,
        payload: json!({
            "truncated": report.truncated,
            "inputTokens": report.input_tokens,
            "outputTokens": report.output_tokens,
            "elapsedMs": report.elapsed_ms,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Echo;

    #[async_trait::async_trait]
    impl Tool for Echo {
        fn id(&self) -> &str {
            "echo"
        }

        async fn execute(
            &self,
            _input: Value,
            _ctx: &ToolContext<'_>,
        ) -> Result<Value, KernelError> {
            Ok(json!({"ok": true}))
        }
    }

    #[tokio::test]
    async fn builder_collects_registrations() {
        let kernel = Kernel::builder().tool(Echo).build().unwrap();
        assert_eq!(kernel.tool_registry().ids(), vec!["echo"]);
        let out = kernel.call_tool("echo", json!({})).await.unwrap();
        assert_eq!(out["ok"], true);
        assert!(matches!(
            kernel.call_tool("missing", json!({})).await,
            Err(KernelError::ToolNotFound(_))
        ));
    }

    #[tokio::test]
    async fn service_roundtrip() {
        let kernel = Kernel::builder().service(Arc::new(7_u32)).build().unwrap();
        let value = kernel.service::<u32>().unwrap();
        assert_eq!(*value, 7);
        assert!(matches!(
            kernel.service::<u64>(),
            Err(KernelError::ServiceNotFound(_))
        ));
    }
}
