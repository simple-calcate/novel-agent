//! 手动入队（对应前端 enqueue_job 命令）端到端流转：
//! 无需工作流规则，直接入队 -> queue.tick 执行 -> list_jobs 看到状态推进。

use chrono::Duration;
use novel_extensions::{QueueExtension, QueuePolicy};
use novel_kernel::{Kernel, Tool};
use novel_storage::StorageHandle;
use serde_json::json;
use std::sync::Arc;

struct OkSaveTool;

#[async_trait::async_trait]
impl Tool for OkSaveTool {
    fn id(&self) -> &str {
        "document.save"
    }

    async fn execute(
        &self,
        _input: serde_json::Value,
        _ctx: &novel_kernel::ToolContext<'_>,
    ) -> Result<serde_json::Value, novel_kernel::KernelError> {
        Ok(json!({"saved": true}))
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
        .tool(OkSaveTool)
        .build()
        .unwrap()
}

#[tokio::test]
async fn manual_enqueue_runs_and_succeeds() {
    let storage = Arc::new(StorageHandle::open_in_memory().unwrap());
    let project_id = storage
        .execute(|repository| repository.create_project("测试作品"))
        .unwrap()
        .id;
    let kernel = build_kernel(storage.clone());

    // 模拟前端 enqueue_job：直接构造 Job 入队（跳过工作流匹配）
    let now = chrono::Utc::now();
    let job = novel_domain::Job {
        id: Default::default(),
        project_id: project_id.clone(),
        workflow_id: None,
        operation: "document.save".into(),
        payload: json!({"projectId": project_id.to_string()}),
        priority: 0,
        status: novel_domain::JobStatus::Pending,
        idempotency_key: format!("manual:{}", uuid::Uuid::new_v4()),
        depends_on: vec![],
        attempts: 0,
        max_attempts: 3,
        run_at: now,
        deadline: None,
        causation_id: None,
        causation_depth: 0,
        created_at: now,
        updated_at: now,
    };
    assert!(storage
        .execute(|repository| repository.enqueue_job(&job))
        .unwrap());

    // 入队后 list_jobs 显示 pending
    let pending = storage
        .execute(|repository| repository.list_jobs(10))
        .unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].status, novel_domain::JobStatus::Pending);

    // queue.tick 领取并执行成功
    let result = kernel.call_tool("queue.tick", json!({})).await.unwrap();
    assert_eq!(result["executed"], true);
    assert_eq!(result["status"], "succeeded");

    // list_jobs 显示 succeeded
    let done = storage
        .execute(|repository| repository.list_jobs(10))
        .unwrap();
    assert_eq!(done[0].status, novel_domain::JobStatus::Succeeded);
    assert_eq!(done[0].attempts, 1);

    // 队列已空
    let result = kernel.call_tool("queue.tick", json!({})).await.unwrap();
    assert_eq!(result["executed"], false);
}
