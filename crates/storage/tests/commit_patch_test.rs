//! 写路径端到端：project → book → chapter → commit_patch → 版本递增、
//! 正文落库、操作日志归属正确项目。

use novel_domain::{
    Actor, BlockId, ChapterId, ContentPatch, ProjectId, ProposalId, Revision, TextOperation,
};
use novel_storage::Repository;

fn setup() -> (Repository, ChapterId, ProjectId) {
    let repository = Repository::open_in_memory().unwrap();
    let project = repository.create_project("夜航星图").unwrap();
    let book = repository
        .create_book(&project.id, "卷一 · 雾与海", "雾港悬疑线", 1)
        .unwrap();
    let chapter = repository
        .create_chapter(&project.id, &book.id.to_string(), "第一章 雾港来客", 1)
        .unwrap();
    (repository, chapter.id, project.id)
}

fn insert_patch(chapter_id: ChapterId, base: u64, text: &str) -> ContentPatch {
    ContentPatch {
        id: ProposalId::new(),
        chapter_id,
        base_revision: Revision(base),
        operations: vec![TextOperation::Insert {
            block_id: BlockId::new(),
            offset: u32::MAX, // 追加到文末
            text: text.into(),
        }],
        rationale: "测试补丁".into(),
        created_by: Actor::Agent {
            model: "test".into(),
        },
        created_at: chrono::Utc::now(),
    }
}

#[test]
fn create_book_and_chapter_happy_path() {
    let (repository, chapter_id, project_id) = setup();
    assert_eq!(
        repository.current_revision(&chapter_id).unwrap(),
        Revision::INITIAL
    );
    assert_eq!(
        repository.chapter_project_id(&chapter_id).unwrap(),
        Some(project_id.clone())
    );

    // book 属于另一个项目，挂到本项目名下会被拒绝
    let other_project = repository.create_project("另一部").unwrap();
    let book_b = repository
        .create_book(&other_project.id, "张冠李戴", "", 1)
        .unwrap();
    let error = repository
        .create_chapter(&project_id, &book_b.id.to_string(), "越权章节", 1)
        .unwrap_err();
    assert!(error.to_string().contains("not found"), "{error}");
}

#[test]
fn library_lists_and_snapshot_roundtrip() {
    let mut repository = Repository::open_in_memory().unwrap();
    let project = repository.create_project("夜航星图").unwrap();
    repository.create_book(&project.id, "卷一", "", 0).unwrap();
    let book_2 = repository
        .create_book(&project.id, "卷二", "续篇", 0)
        .unwrap();
    let books = repository.list_books(&project.id).unwrap();
    assert_eq!(books.len(), 2);
    assert_eq!(books[0].position, 1);
    assert_eq!(books[1].position, 2);

    let chapter = repository
        .create_chapter(&project.id, &book_2.id.to_string(), "第一章", 0)
        .unwrap();
    let next = repository
        .save_chapter_snapshot(&chapter.id, "雾在潮响前漫进港口。", "user")
        .unwrap();
    assert_eq!(next, Revision(1));
    let same = repository
        .save_chapter_snapshot(&chapter.id, "雾在潮响前漫进港口。", "user")
        .unwrap();
    assert_eq!(same, Revision(1));

    let listed = repository.list_chapters(&project.id).unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].title, "第一章");
    assert!(repository
        .list_projects()
        .unwrap()
        .iter()
        .any(|item| item.title == "夜航星图"));
}

#[test]
fn rename_delete_and_move_library_items() {
    let repository = Repository::open_in_memory().unwrap();
    let project = repository.create_project("夜航星图").unwrap();
    let book_a = repository.create_book(&project.id, "卷一", "", 0).unwrap();
    let book_b = repository.create_book(&project.id, "卷二", "", 0).unwrap();
    let chapter = repository
        .create_chapter(&project.id, &book_a.id.to_string(), "第一章", 0)
        .unwrap();
    repository
        .create_chapter(&project.id, &book_a.id.to_string(), "第二章", 0)
        .unwrap();

    let renamed = repository
        .rename_book(&project.id, &book_a.id, "卷一 · 改名")
        .unwrap();
    assert_eq!(renamed.title, "卷一 · 改名");
    repository
        .rename_chapter(&project.id, &chapter.id, "序章")
        .unwrap();
    let moved = repository.move_book(&project.id, &book_a.id, 1).unwrap();
    assert_eq!(moved[0].id, book_b.id);
    assert_eq!(moved[1].title, "卷一 · 改名");

    let chapters = repository
        .move_chapter(&project.id, &chapter.id, 1)
        .unwrap();
    let in_book: Vec<_> = chapters
        .iter()
        .filter(|item| item.book_id == book_a.id)
        .collect();
    assert_eq!(in_book[0].title, "第二章");
    assert_eq!(in_book[1].title, "序章");

    repository.delete_chapter(&project.id, &chapter.id).unwrap();
    assert_eq!(
        repository
            .list_chapters(&project.id)
            .unwrap()
            .iter()
            .filter(|item| item.book_id == book_a.id)
            .count(),
        1
    );
    repository.delete_book(&project.id, &book_a.id).unwrap();
    assert_eq!(repository.list_books(&project.id).unwrap().len(), 1);

    repository.rename_project(&project.id, "改名作品").unwrap();
    repository.delete_project(&project.id).unwrap();
    assert!(repository.list_projects().unwrap().is_empty());
}

#[test]
fn block_sequence_roundtrip_and_skip_unchanged() {
    let mut repository = Repository::open_in_memory().unwrap();
    let project = repository.create_project("夜航星图").unwrap();
    let book = repository.create_book(&project.id, "卷一", "", 0).unwrap();
    let chapter = repository
        .create_chapter(&project.id, &book.id.to_string(), "第一章", 0)
        .unwrap();
    let thinking = novel_domain::ContentBlock {
        id: BlockId::new(),
        kind: novel_domain::BlockKind::Thinking,
        text: "先写动机".into(),
        position: 0,
        markup: vec![],
    };
    let body = novel_domain::ContentBlock {
        id: BlockId::new(),
        kind: novel_domain::BlockKind::Body,
        text: "雾在潮响前漫进港口。".into(),
        position: 1,
        markup: vec![],
    };
    let next = repository
        .save_block_sequence(&chapter.id, &[thinking.clone(), body.clone()])
        .unwrap();
    assert_eq!(next, Revision(1));
    let loaded = repository
        .block_sequence(&chapter.id, next)
        .unwrap()
        .unwrap();
    assert_eq!(loaded.blocks[0].kind, novel_domain::BlockKind::Thinking);
    assert_eq!(loaded.blocks[1].text, "雾在潮响前漫进港口。");
    assert_eq!(
        repository.chapter_text(&chapter.id, next).unwrap(),
        Some("雾在潮响前漫进港口。".into())
    );

    let same = repository
        .save_block_sequence(&chapter.id, &[thinking, body])
        .unwrap();
    assert_eq!(same, Revision(1));
}

#[test]
fn create_book_rejects_unknown_project() {
    let repository = Repository::open_in_memory().unwrap();
    let missing = ProjectId::new();
    let error = repository
        .create_book(&missing, "幽灵书", "", 1)
        .unwrap_err();
    assert!(error.to_string().contains("not found"), "{error}");
}

#[test]
fn commit_patch_appends_revision_and_logs_project() {
    let (mut repository, chapter_id, _) = setup();

    let patch = insert_patch(chapter_id.clone(), 0, "雾在潮响前漫进港口。");
    let next = repository
        .commit_patch(&patch, "author", &patch.operations)
        .unwrap();
    assert_eq!(next, Revision(1));
    assert_eq!(
        repository.current_revision(&chapter_id).unwrap(),
        Revision(1)
    );
    assert_eq!(
        repository.chapter_text(&chapter_id, Revision(1)).unwrap(),
        Some("雾在潮响前漫进港口。".into())
    );

    // 第二次提交基于 revision 1，追加而非覆盖
    let patch = insert_patch(chapter_id.clone(), 1, "灯第三次亮起。");
    let next = repository
        .commit_patch(&patch, "author", &patch.operations)
        .unwrap();
    assert_eq!(next, Revision(2));
    assert_eq!(
        repository.chapter_text(&chapter_id, Revision(2)).unwrap(),
        Some("雾在潮响前漫进港口。灯第三次亮起。".into())
    );

    // 操作日志的 project_id 是真实项目，而不是空字符串
    let project_ids = repository.operation_log_project_ids(&chapter_id).unwrap();
    assert_eq!(project_ids.len(), 2);
    for project_id in &project_ids {
        assert!(!project_id.is_empty(), "project_id 不应为空");
        assert!(uuid::Uuid::parse_str(project_id).is_ok(), "应为合法 UUID");
    }
}

#[test]
fn commit_patch_rejects_stale_base_revision() {
    let (mut repository, chapter_id, _) = setup();
    let patch = insert_patch(chapter_id.clone(), 0, "第一版");
    repository
        .commit_patch(&patch, "author", &patch.operations)
        .unwrap();

    // 基于过期的 base_revision 0 再次提交 → 版本冲突
    let stale = insert_patch(chapter_id.clone(), 0, "冲突版");
    let result = repository.commit_patch(&stale, "author", &stale.operations);
    let error = result.unwrap_err();
    assert!(error.to_string().contains("revision conflict"), "{error}");
    // 版本未被破坏
    assert_eq!(
        repository.current_revision(&chapter_id).unwrap(),
        Revision(1)
    );
}

#[test]
fn commit_patch_unknown_chapter_fails() {
    let (mut repository, _, _) = setup();
    assert!(repository
        .commit_patch(
            &insert_patch(ChapterId::new(), 0, "幽灵章节"),
            "author",
            &[]
        )
        .is_err());
}
