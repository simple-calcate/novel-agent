//! 工作流引擎扩展：订阅领域事件 → 记录事件 → 匹配工作流规则
//! （含冷却检查）→ 以稳定幂等键把动作入队。

use crate::util::repository;
use chrono::Utc;
use novel_automation::{operation_name, rule_matches, stable_idempotency_key, JobQueue};
use novel_domain::DomainEvent;
use novel_kernel::{EventSubscriber, Extension, Kernel, KernelBuilder, KernelError};
use serde_json::{json, Value};

pub struct WorkflowEngineSubscriber;

impl EventSubscriber for WorkflowEngineSubscriber {
    fn id(&self) -> &str {
        "builtin.workflow-engine"
    }

    fn handle(&self, kernel: &Kernel, event: &DomainEvent) -> Result<Value, KernelError> {
        let repository = repository(kernel)?;
        let guard = repository
            .lock()
            .map_err(|_| KernelError::Storage("repository mutex poisoned".into()))?;
        guard
            .record_event(event)
            .map_err(|e| KernelError::Storage(e.to_string()))?;

        let rules = guard
            .workflows_for_event(&event.project_id, &event.event_type)
            .map_err(|e| KernelError::Storage(e.to_string()))?;

        let now = Utc::now();
        let queue = JobQueue::new(&guard);
        let mut queued = 0usize;
        let mut skipped_by_cooldown = 0usize;

        for rule in rules.iter().filter(|rule| rule_matches(rule, event)) {
            let in_cooldown = guard
                .workflow_in_cooldown(rule, &event.event_type, now)
                .map_err(|e| KernelError::Storage(e.to_string()))?;
            if in_cooldown {
                skipped_by_cooldown += 1;
                continue;
            }
            guard
                .record_workflow_fired(&rule.id, &event.event_type, now)
                .map_err(|e| KernelError::Storage(e.to_string()))?;

            for action in &rule.actions {
                // 幂等键 = 事件 + 工作流 + 动作名：同一事件重复投递不会重复入队。
                let operation = operation_name(action);
                let key =
                    stable_idempotency_key(&event.event_id.to_string(), &rule.id.0, operation);
                let enqueued = queue
                    .enqueue(
                        event.project_id.clone(),
                        Some(rule.id.clone()),
                        action,
                        event.payload.clone(),
                        rule.priority,
                        key,
                        event
                            .causation_id
                            .clone()
                            .or_else(|| Some(event.event_id.to_string())),
                        1,
                    )
                    .map_err(|e| KernelError::Storage(e.to_string()))?;
                if enqueued.is_some() {
                    queued += 1;
                }
            }
        }

        Ok(json!({
            "recorded": true,
            "queued": queued,
            "skippedByCooldown": skipped_by_cooldown,
        }))
    }
}

/// 工作流引擎扩展：注册事件订阅者。
pub struct WorkflowEngineExtension;

impl Extension for WorkflowEngineExtension {
    fn id(&self) -> &str {
        "builtin.workflow"
    }

    fn setup(&self, builder: &mut KernelBuilder) -> Result<(), KernelError> {
        builder.add_subscriber(WorkflowEngineSubscriber);
        Ok(())
    }
}
