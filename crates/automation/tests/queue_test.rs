use novel_automation::{JobQueue, QueueError};
use novel_domain::{ProjectId, WorkflowAction, WorkflowId};
use novel_storage::Repository;
use serde_json::json;

#[test]
fn enqueue_and_retrieve_job() {
    let repo = Repository::open_in_memory().unwrap();
    let queue = JobQueue::new(&repo);

    let job_id = queue
        .enqueue(
            ProjectId::new(),
            Some(WorkflowId::new()),
            &WorkflowAction::SaveDocument,
            json!({"chapterId": "c1"}),
            10,
            "key-1".into(),
            None,
            0,
        )
        .unwrap();

    assert!(job_id.is_some());

    let job = queue.next_due().unwrap();
    assert!(job.is_some());
    let job = job.unwrap();
    assert_eq!(job.operation, "document.save");
    assert_eq!(job.priority, 10);
}

#[test]
fn idempotency_prevents_duplicate() {
    let repo = Repository::open_in_memory().unwrap();
    let queue = JobQueue::new(&repo);
    let project_id = ProjectId::new();

    let first = queue
        .enqueue(
            project_id.clone(),
            None,
            &WorkflowAction::SaveDocument,
            json!({}),
            0,
            "same-key".into(),
            None,
            0,
        )
        .unwrap();
    let second = queue
        .enqueue(
            project_id,
            None,
            &WorkflowAction::SaveDocument,
            json!({}),
            0,
            "same-key".into(),
            None,
            0,
        )
        .unwrap();

    assert!(first.is_some());
    assert!(second.is_none()); // 幂等键冲突，被忽略
}

#[test]
fn causation_depth_exceeded() {
    let repo = Repository::open_in_memory().unwrap();
    let queue = JobQueue::new(&repo);

    let result = queue.enqueue(
        ProjectId::new(),
        None,
        &WorkflowAction::SaveDocument,
        json!({}),
        0,
        "deep".into(),
        Some("parent".into()),
        9, // 超过 MAX_CAUSATION_DEPTH
    );

    assert!(matches!(result, Err(QueueError::CausationDepthExceeded)));
}
