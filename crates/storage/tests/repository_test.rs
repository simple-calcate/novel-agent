use novel_domain::{AnnotationAnchor, BlockId, Revision, TextOperation};
use novel_storage::Repository;

#[test]
fn apply_insert_operation() {
    let mut text = String::from("雾在潮响前漫进港口。");
    let op = TextOperation::Insert {
        block_id: BlockId::new(),
        offset: 0,
        text: "清晨，".into(),
    };
    novel_storage::apply_operation_for_test(&mut text, &op).unwrap();
    assert_eq!(text, "清晨，雾在潮响前漫进港口。");
}

#[test]
fn apply_delete_operation() {
    let mut text = String::from("清晨，雾在潮响前漫进港口。");
    // 删除前 3 个字符（"清晨，"），每个中文字符占 3 字节 UTF-8
    let op = TextOperation::Delete {
        block_id: BlockId::new(),
        offset: 0,
        length: 9,
    };
    novel_storage::apply_operation_for_test(&mut text, &op).unwrap();
    assert_eq!(text, "雾在潮响前漫进港口。");
}

#[test]
fn revision_conflict_error_message() {
    let err = novel_domain::DomainError::RevisionConflict {
        expected: 1,
        actual: 2,
    };
    assert!(err.to_string().contains("revision conflict"));
}

#[test]
fn annotation_anchor_roundtrip() {
    let anchor = AnnotationAnchor {
        block_id: BlockId::new(),
        base_revision: Revision(1),
        start_offset: 0,
        end_offset: 5,
        quote: "雾在潮响前".into(),
        prefix: "".into(),
        suffix: "漫进港口".into(),
    };
    let json = serde_json::to_string(&anchor).unwrap();
    let parsed: AnnotationAnchor = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.quote, "雾在潮响前");
    assert_eq!(parsed.base_revision, Revision(1));
}

#[test]
fn in_memory_repository_opens() {
    let repo = Repository::open_in_memory();
    assert!(repo.is_ok());
}
