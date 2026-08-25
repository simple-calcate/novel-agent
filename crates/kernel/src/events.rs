use crate::{Kernel, KernelError};
use novel_domain::DomainEvent;
use serde_json::Value;
use std::sync::Arc;

/// 事件订阅者：对领域事件做出反应（记录、匹配工作流、入队等）。
/// 单个订阅者失败不会中断其他订阅者。
pub trait EventSubscriber: Send + Sync {
    fn id(&self) -> &str;

    /// 关心的事件类型；空切片表示订阅全部事件。
    fn event_types(&self) -> &[&str] {
        &[]
    }

    fn handle(&self, kernel: &Kernel, event: &DomainEvent) -> Result<Value, KernelError>;
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriberOutcome {
    pub subscriber: String,
    pub output: Option<Value>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DispatchSummary {
    pub outcomes: Vec<SubscriberOutcome>,
}

impl DispatchSummary {
    pub fn has_error(&self) -> bool {
        self.outcomes.iter().any(|o| o.error.is_some())
    }

    pub fn first_error(&self) -> Option<&str> {
        self.outcomes
            .iter()
            .find(|o| o.error.is_some())
            .and_then(|o| o.error.as_deref())
    }
}

#[derive(Default)]
pub struct EventBus {
    subscribers: Vec<Arc<dyn EventSubscriber>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn with_subscribers(subscribers: Vec<Arc<dyn EventSubscriber>>) -> Self {
        Self { subscribers }
    }

    pub fn subscribe(&mut self, subscriber: impl EventSubscriber + 'static) {
        self.subscribers.push(Arc::new(subscriber));
    }

    pub fn dispatch(&self, kernel: &Kernel, event: &DomainEvent) -> DispatchSummary {
        let mut summary = DispatchSummary::default();
        for subscriber in &self.subscribers {
            let interested = subscriber.event_types().is_empty()
                || subscriber
                    .event_types()
                    .iter()
                    .any(|kind| *kind == event.event_type);
            if !interested {
                continue;
            }
            let outcome = match subscriber.handle(kernel, event) {
                Ok(output) => SubscriberOutcome {
                    subscriber: subscriber.id().to_owned(),
                    output: Some(output),
                    error: None,
                },
                Err(error) => SubscriberOutcome {
                    subscriber: subscriber.id().to_owned(),
                    output: None,
                    error: Some(error.to_string()),
                },
            };
            summary.outcomes.push(outcome);
        }
        summary
    }
}
