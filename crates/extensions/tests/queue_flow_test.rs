//! 端到端流程：领域事件 → 工作流匹配 → 入队 → queue.tick 执行 →
//! 状态机推进（succeeded / 重试 / 死信），以及冷却与幂等。

use async_trait::async_trait;
use chrono::Duration;
use novel_domain::{
    Actor, DomainEvent, EventId, EventSource, Platform, ProjectId, Revision, WorkflowAction,
    WorkflowRule, WorkflowTrigger, EVENT_SCHEMA_VERSION,
};
use novel_extensions::{QueueExtension, QueuePolicy, WorkflowEngineExtension};
use novel_kernel::{Kernel, KernelError, Tool, ToolContext};
use novel_storage::Repository;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

struct OkSaveTool;

#[async_trait]
impl Tool for OkSaveTool {
    fn id(&self) -> &str {
        "document.save"
    }

    async fn execute(&self, _input: Value, _ctx: &ToolContext<'_>) -> Result<Value, KernelError> {
        Ok(json!({"saved": true}))
    }
}

struct FailingTool;

#[async_trait]
impl Tool for FailingTool {
    fn id(&self) -> &str {
        "index.rebuild"
    }

    async fn execute(&self, _input: Value, _ctx: &ToolContext<'_>) -> Result<Value, KernelError> {
        Err(KernelError::Storage("模拟失败".into()))
    }
}

fn rule(
    project_id: ProjectId,
    event_type: &str,
    action: WorkflowAction,
    cooldown_ms: u64,
) -> WorkflowRule {
    WorkflowRule {
        id: Default::default(),
        project_id,
        name: "测试规则".into(),
        enabled: true,
        trigger: WorkflowTrigger {
            event_type: event_type.into(),
        },
        conditions: vec![],
        actions: vec![action],
        priority: 100,
        cooldown_ms,
    }
}

fn event(project_id: ProjectId, event_type: &str) -> DomainEvent {
    let payload_project = project_id.to_string();
    DomainEvent {
        event_id: EventId::new(),
        event_type: event_type.into(),
        schema_version: EVENT_SCHEMA_VERSION,
        occurred_at: chrono::Utc::now(),
        project_id,
        book_id: None,
        chapter_id: None,
        scene_id: None,
        block_id: None,
        actor: Actor::User { user_id: None },
        source: EventSource::Editor,
        platform: Platform::Linux,
        transaction_id: Uuid::new_v4().to_string(),
        correlation_id: None,
        causation_id: None,
        revision_before: Revision(0),
        revision_after: Revision(1),
        payload: json!({"projectId": payload_project}),
    }
}

fn build_kernel(
    repository: Arc<Mutex<Repository>>,
    extra_tool: Option<Box<dyn Tool + 'static>>,
) -> Kernel {
    let mut builder = Kernel::builder()
        .service(repository)
        .service(Arc::new(QueuePolicy {
            stale_running_after: Duration::minutes(10),
            backoff_base: Duration::zero(),
        }))
        .extension(WorkflowEngineExtension)
        .unwrap()
        .extension(QueueExtension)
        .unwrap();
    if let Some(tool) = extra_tool {
        builder = builder.tool(tool);
    }
    builder.build().unwrap()
}

fn queued_count(summary: &novel_kernel::DispatchSummary) -> u64 {
    summary
        .outcomes
        .iter()
        .filter_map(|outcome| {
            outcome
                .output
                .as_ref()
                .and_then(|output| output.get("queued"))
                .and_then(Value::as_u64)
        })
        .sum()
}

fn skipped_count(summary: &novel_kernel::DispatchSummary) -> u64 {
    summary
        .outcomes
        .iter()
        .filter_map(|outcome| {
            outcome
                .output
                .as_ref()
                .and_then(|output| output.get("skippedByCooldown"))
                .and_then(Value::as_u64)
        })
        .sum()
}

#[tokio::test]
async fn event_to_queue_to_success() {
    let repository = Arc::new(Mutex::new(Repository::open_in_memory().unwrap()));
    let project_id = repository
        .lock()
        .unwrap()
        .create_project("测试作品")
        .unwrap()
        .id;
    repository
        .lock()
        .unwrap()
        .save_workflow(&rule(
            project_id.clone(),
            "editor.idle",
            WorkflowAction::SaveDocument,
            0,
        ))
        .unwrap();

    let kernel = build_kernel(repository.clone(), Some(Box::new(OkSaveTool)));

    let summary = kernel.dispatch(&event(project_id.clone(), "editor.idle"));
    assert!(!summary.has_error(), "{summary:?}");
    assert_eq!(queued_count(&summary), 1);

    let result = kernel.call_tool("queue.tick", json!({})).await.unwrap();
    assert_eq!(result["executed"], true);
    assert_eq!(result["success"], true);
    assert_eq!(result["operation"], "document.save");
    assert_eq!(result["status"], "succeeded");

    let jobs = repository.lock().unwrap().list_jobs(10).unwrap();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].status, novel_domain::JobStatus::Succeeded);
    assert_eq!(jobs[0].attempts, 1);

    // 队列已空
    let result = kernel.call_tool("queue.tick", json!({})).await.unwrap();
    assert_eq!(result["executed"], false);
}

#[tokio::test]
async fn failing_tool_retries_then_dead_letters() {
    let repository = Arc::new(Mutex::new(Repository::open_in_memory().unwrap()));
    let project_id = repository
        .lock()
        .unwrap()
        .create_project("测试作品")
        .unwrap()
        .id;
    repository
        .lock()
        .unwrap()
        .save_workflow(&rule(
            project_id.clone(),
            "editor.idle",
            WorkflowAction::RebuildIndex,
            0,
        ))
        .unwrap();

    let kernel = build_kernel(repository.clone(), Some(Box::new(FailingTool)));
    kernel.dispatch(&event(project_id.clone(), "editor.idle"));

    // max_attempts = 3：两次失败回到 pending，第三次进死信
    for attempt in 1..=3u32 {
        let result = kernel.call_tool("queue.tick", json!({})).await.unwrap();
        assert_eq!(result["success"], false);
        assert_eq!(result["attempts"], attempt);
        let expected_status = if attempt == 3 {
            "deadLetter"
        } else {
            "pending"
        };
        assert_eq!(result["status"], expected_status, "第 {attempt} 次尝试");
    }

    let jobs = repository.lock().unwrap().list_jobs(10).unwrap();
    assert_eq!(jobs[0].status, novel_domain::JobStatus::DeadLetter);
    assert_eq!(jobs[0].attempts, 3);
}

#[tokio::test]
async fn cooldown_blocks_repeat_firing() {
    let repository = Arc::new(Mutex::new(Repository::open_in_memory().unwrap()));
    let project_id = repository
        .lock()
        .unwrap()
        .create_project("测试作品")
        .unwrap()
        .id;
    repository
        .lock()
        .unwrap()
        .save_workflow(&rule(
            project_id.clone(),
            "editor.idle",
            WorkflowAction::SaveDocument,
            3_600_000, // 1 小时冷却
        ))
        .unwrap();

    let kernel = build_kernel(repository.clone(), Some(Box::new(OkSaveTool)));

    let first = kernel.dispatch(&event(project_id.clone(), "editor.idle"));
    assert_eq!(queued_count(&first), 1);
    assert_eq!(skipped_count(&first), 0);

    // 冷却期内的新事件不再入队
    let second = kernel.dispatch(&event(project_id.clone(), "editor.idle"));
    assert!(!second.has_error());
    assert_eq!(queued_count(&second), 0);
    assert_eq!(skipped_count(&second), 1);

    assert_eq!(repository.lock().unwrap().list_jobs(10).unwrap().len(), 1);
}

#[tokio::test]
async fn unknown_operation_is_recorded_as_failure() {
    let repository = Arc::new(Mutex::new(Repository::open_in_memory().unwrap()));
    let project_id = repository
        .lock()
        .unwrap()
        .create_project("测试作品")
        .unwrap()
        .id;
    repository
        .lock()
        .unwrap()
        .save_workflow(&rule(
            project_id.clone(),
            "editor.idle",
            WorkflowAction::CheckContinuity, // 未注册对应工具
            0,
        ))
        .unwrap();

    let kernel = build_kernel(repository, None);
    kernel.dispatch(&event(project_id.clone(), "editor.idle"));

    let result = kernel.call_tool("queue.tick", json!({})).await.unwrap();
    assert_eq!(result["success"], false);
    assert!(result["error"]
        .as_str()
        .unwrap()
        .contains("continuity.check"));
}
