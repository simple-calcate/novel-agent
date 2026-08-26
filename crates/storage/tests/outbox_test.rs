//! Outbox 与业务写在同一事务：成功提交才可见，冲突回滚不留行。

use novel_domain::{Actor, BlockId, ContentPatch, ProposalId, Revision, TextOperation};
use novel_storage::Repository;

#[test]
fn library_writes_enqueue_outbox() {
    let repository = Repository::open_in_memory().unwrap();
    let project = repository.create_project("夜航星图").unwrap();
    let pending = repository.list_pending_outbox(10).unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].event_type, "project.created");
    assert_eq!(pending[0].project_id, project.id.to_string());

    repository.create_book(&project.id, "卷一", "", 0).unwrap();
    let pending = repository.list_pending_outbox(10).unwrap();
    assert_eq!(pending.len(), 2);
    assert_eq!(pending[1].event_type, "book.created");
}

#[test]
fn chapter_revision_and_outbox_share_transaction() {
    let mut repository = Repository::open_in_memory().unwrap();
    let project = repository.create_project("夜航星图").unwrap();
    let book = repository.create_book(&project.id, "卷一", "", 0).unwrap();
    let chapter = repository
        .create_chapter(&project.id, &book.id.to_string(), "第一章", 0)
        .unwrap();
    let before = repository.list_pending_outbox(20).unwrap().len();
    repository
        .save_chapter_snapshot(&chapter.id, "雾港来客。", "user")
        .unwrap();
    let pending = repository.list_pending_outbox(20).unwrap();
    assert_eq!(pending.len(), before + 1);
    assert_eq!(pending.last().unwrap().event_type, "chapter.revised");
    assert_eq!(
        pending.last().unwrap().payload["chapterId"],
        chapter.id.to_string()
    );

    let conflict = ContentPatch {
        id: ProposalId::new(),
        chapter_id: chapter.id.clone(),
        base_revision: Revision(0),
        operations: vec![TextOperation::Insert {
            block_id: BlockId::new(),
            offset: 0,
            text: "不该写入".into(),
        }],
        rationale: "冲突".into(),
        created_by: Actor::User { user_id: None },
        created_at: chrono::Utc::now(),
    };
    let error = repository
        .commit_patch(&conflict, "user", &conflict.operations)
        .unwrap_err();
    assert!(error.to_string().contains("revision conflict"), "{error}");
    assert_eq!(
        repository.list_pending_outbox(20).unwrap().len(),
        pending.len()
    );
}

#[test]
fn mark_delivered_hides_pending_rows() {
    let repository = Repository::open_in_memory().unwrap();
    repository.create_project("夜航星图").unwrap();
    let pending = repository.list_pending_outbox(10).unwrap();
    let updated = repository.mark_outbox_delivered(&[pending[0].id]).unwrap();
    assert_eq!(updated, 1);
    assert!(repository.list_pending_outbox(10).unwrap().is_empty());
}
