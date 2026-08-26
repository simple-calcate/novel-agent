use novel_domain::{EntityKind, ExtractedMention, FactStatus};
use novel_storage::Repository;

#[test]
fn propose_accept_filters_candidates_from_hints_source() {
    let mut repository = Repository::open_in_memory().unwrap();
    let project = repository.create_project("夜航星图").unwrap();
    let book = repository.create_book(&project.id, "卷一", "", 0).unwrap();
    let chapter = repository
        .create_chapter(&project.id, &book.id.to_string(), "第一章", 0)
        .unwrap();
    repository
        .save_chapter_snapshot(&chapter.id, "林晚说道：「今夜雾很重。」", "test")
        .unwrap();

    let revision = repository.current_revision(&chapter.id).unwrap();
    let mentions = vec![ExtractedMention {
        entity_name: "林晚".into(),
        entity_kind: EntityKind::Character,
        predicate: "appearsAsSpeaker".into(),
        object: "林晚".into(),
        quote: "林晚说道".into(),
        confidence: 0.82,
    }];

    let created = repository
        .propose_canon_mentions(&project.id, &chapter.id, revision, &mentions)
        .unwrap();
    assert_eq!(created.len(), 1);
    assert_eq!(created[0].status, FactStatus::Candidate);

    let again = repository
        .propose_canon_mentions(&project.id, &chapter.id, revision, &mentions)
        .unwrap();
    assert!(again.is_empty(), "重复抽取不应再插入");

    let candidates = repository
        .list_canon_proposals(&project.id, Some(FactStatus::Candidate))
        .unwrap();
    assert_eq!(candidates.len(), 1);

    let accepted_before = repository
        .list_canon_facts_for_project(&project.id, Some(FactStatus::Accepted))
        .unwrap();
    assert!(accepted_before.is_empty());

    repository
        .set_fact_status(&candidates[0].fact_id, FactStatus::Accepted)
        .unwrap();

    let accepted = repository
        .list_canon_proposals(&project.id, Some(FactStatus::Accepted))
        .unwrap();
    assert_eq!(accepted.len(), 1);
    assert_eq!(accepted[0].entity_name, "林晚");

    let indexed = repository.rebuild_search_index(&project.id).unwrap();
    assert_eq!(indexed, 1);

    let designed = repository.list_story_entries(&project.id).unwrap();
    assert!(designed.is_empty(), "抽取确认不应进入预先结构");
}
