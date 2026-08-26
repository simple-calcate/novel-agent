//! block.mode.changed 信号量端到端链路：
//! 前端 Tab 切换思考/正文 -> emit_block_mode_changed 命令 -> kernel.dispatch 领域事件
//! -> 工作流规则匹配 -> 任务序列入队 -> queue.tick 执行成功。
//! 验证"每一个信号量触发都会处理与之对应的任务序列"的插件扩展点。

use chrono::Duration;
use novel_domain::{
    Actor, ChapterId, DomainEvent, EventId, EventSource, Platform, Revision, WorkflowAction,
    WorkflowRule, WorkflowTrigger, EVENT_SCHEMA_VERSION,
};
use novel_extensions::{QueueExtension, QueuePolicy, WorkflowEngineExtension};
use novel_kernel::{Kernel, Tool};
use novel_storage::StorageHandle;
use serde_json::json;
use std::sync::Arc;

struct RebuildIndexTool;

#[async_trait::async_trait]
impl Tool for RebuildIndexTool {
    fn id(&self) -> &str {
        "index.rebuild"
    }

    async fn execute(
        &self,
        _input: serde_json::Value,
        _ctx: &novel_kernel::ToolContext<'_>,
    ) -> Result<serde_json::Value, novel_kernel::KernelError> {
        Ok(json!({"indexed": true}))
    }
}

fn build_kernel(storage: Arc<StorageHandle>) -> Kernel {
    Kernel::builder()
        .service(storage)
        .service(Arc::new(QueuePolicy {
            stale_running_after: Duration::minutes(10),
            backoff_base: Duration::zero(),
        }))
        .extension(QueueExtension)
        .unwrap()
        .extension(WorkflowEngineExtension)
        .unwrap()
        .tool(RebuildIndexTool)
        .build()
        .unwrap()
}

#[tokio::test]
async fn block_mode_changed_queues_workflow_sequence() {
    let storage = Arc::new(StorageHandle::open_in_memory().unwrap());
    let project_id = storage
        .execute(|repository| repository.create_project("测试作品"))
        .unwrap()
        .id;

    // 插件扩展点：注册一条监听 block.mode.changed 的工作流规则
    let rule = WorkflowRule {
        id: Default::default(),
        project_id: project_id.clone(),
        name: "切到思考模式后重建索引".into(),
        enabled: true,
        trigger: WorkflowTrigger {
            event_type: "block.mode.changed".into(),
        },
        conditions: vec![],
        actions: vec![WorkflowAction::RebuildIndex],
        priority: 0,
        cooldown_ms: 0,
    };
    storage
        .execute(|repository| repository.save_workflow(&rule))
        .unwrap();

    let kernel = build_kernel(storage.clone());

    // 构造前端 Tab 切换信号（对应 emit_block_mode_changed 的 dispatch 部分）
    let event = DomainEvent {
        event_id: EventId::new(),
        event_type: "block.mode.changed".into(),
        schema_version: EVENT_SCHEMA_VERSION,
        occurred_at: chrono::Utc::now(),
        project_id: project_id.clone(),
        book_id: None,
        chapter_id: Some(ChapterId::new()),
        scene_id: None,
        block_id: None,
        actor: Actor::User { user_id: None },
        source: EventSource::Editor,
        platform: Platform::Unknown,
        transaction_id: format!("mode:{}", EventId::new()),
        correlation_id: None,
        causation_id: None,
        revision_before: Revision::INITIAL,
        revision_after: Revision::INITIAL,
        payload: json!({
            "mode": "thinking",
            "previousMode": "body",
            "blockId": null,
            "position": 42,
        }),
    };

    // 信号量触发：dispatch -> 工作流匹配 -> 入队
    let summary = kernel.dispatch(&event);
    assert!(summary.first_error().is_none());
    let queued = summary.queued_count();
    assert_eq!(queued, 1, "规则应入队 1 个任务");

    // 任务在队列中，携带规则与事件载荷
    let jobs = storage
        .execute(|repository| repository.list_jobs(10))
        .unwrap();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].operation, "index.rebuild");
    assert_eq!(jobs[0].status, novel_domain::JobStatus::Pending);
    assert_eq!(
        jobs[0].workflow_id.as_ref().map(|w| w.to_string()),
        Some(rule.id.to_string())
    );
    assert_eq!(jobs[0].payload["mode"], "thinking");

    // 队列执行任务序列 -> 成功
    let result = kernel.call_tool("queue.tick", json!({})).await.unwrap();
    assert_eq!(result["executed"], true);
    assert_eq!(result["status"], "succeeded");

    let done = storage
        .execute(|repository| repository.list_jobs(10))
        .unwrap();
    assert_eq!(done[0].status, novel_domain::JobStatus::Succeeded);
    assert_eq!(done[0].attempts, 1);
}
