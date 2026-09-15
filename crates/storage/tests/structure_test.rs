use novel_domain::StoryEntryKind;
use novel_storage::Repository;

#[test]
fn designed_story_entries_are_independent_of_canon_extract() {
    let repository = Repository::open_in_memory().unwrap();
    let project = repository.create_project("夜航星图").unwrap();

    let character = repository
        .create_story_entry(
            &project.id,
            StoryEntryKind::Character,
            "林晚",
            "雾港来的刀客",
        )
        .unwrap();
    assert_eq!(character.title, "林晚");
    repository
        .create_story_entry(
            &project.id,
            StoryEntryKind::Foreshadow,
            "雾中灯塔",
            "里面还有旧王玺",
        )
        .unwrap();

    let listed = repository.list_story_entries(&project.id).unwrap();
    assert_eq!(
        listed
            .iter()
            .map(|entry| (entry.kind, entry.title.as_str()))
            .collect::<Vec<_>>(),
        vec![
            (StoryEntryKind::Character, "林晚"),
            (StoryEntryKind::Foreshadow, "雾中灯塔"),
        ]
    );

    let duplicate =
        repository.create_story_entry(&project.id, StoryEntryKind::Character, "林晚", "重复");
    assert!(duplicate.is_err());

    repository
        .delete_story_entry(&project.id, &listed[0].id, listed[0].kind)
        .unwrap();
    let leftover = repository.list_story_entries(&project.id).unwrap();
    assert_eq!(leftover.len(), 1);
    assert_eq!(leftover[0].title, "雾中灯塔");
}

#[test]
fn updates_story_entry_title_aliases_and_summary() {
    let repository = Repository::open_in_memory().unwrap();
    let project = repository.create_project("夜航星图").unwrap();
    let entry = repository
        .create_story_entry(
            &project.id,
            StoryEntryKind::Character,
            "林晚",
            "雾港来的刀客",
        )
        .unwrap();
    repository
        .create_story_entry(&project.id, StoryEntryKind::Character, "沈雾", "码头更夫")
        .unwrap();

    let updated = repository
        .update_story_entry(
            &project.id,
            &entry.id,
            StoryEntryKind::Character,
            "林晚、雾儿",
            "雾港来的刀客，不爱回头",
        )
        .unwrap();
    assert_eq!(updated.title, "林晚");
    assert_eq!(updated.aliases, vec!["雾儿".to_string()]);
    assert_eq!(updated.summary, "雾港来的刀客，不爱回头");

    let listed = repository.list_story_entries(&project.id).unwrap();
    let lin = listed.iter().find(|item| item.id == entry.id).unwrap();
    assert_eq!(lin.aliases, vec!["雾儿".to_string()]);
    assert_eq!(lin.summary, "雾港来的刀客，不爱回头");

    let clash = repository.update_story_entry(
        &project.id,
        &entry.id,
        StoryEntryKind::Character,
        "沈雾",
        "撞名",
    );
    assert!(clash.is_err());

    let missing = repository.update_story_entry(
        &project.id,
        "missing",
        StoryEntryKind::Character,
        "林晚",
        "",
    );
    assert!(missing.is_err());
}

#[test]
fn rebuild_search_index_includes_story_entries() {
    let repository = Repository::open_in_memory().unwrap();
    let project = repository.create_project("夜航星图").unwrap();
    repository
        .create_story_entry(
            &project.id,
            StoryEntryKind::Character,
            "灯塔守夜人",
            "负责在雾季敲钟",
        )
        .unwrap();
    let indexed = repository.rebuild_search_index(&project.id).unwrap();
    assert_eq!(indexed, 1);
}

#[test]
fn splits_aliases_from_title() {
    let repository = Repository::open_in_memory().unwrap();
    let project = repository.create_project("夜航星图").unwrap();
    let entry = repository
        .create_story_entry(
            &project.id,
            StoryEntryKind::Character,
            "林晚、雾儿",
            "雾港来的刀客",
        )
        .unwrap();
    assert_eq!(entry.title, "林晚");
    assert_eq!(entry.aliases, vec!["雾儿".to_string()]);
    let listed = repository.list_story_entries(&project.id).unwrap();
    assert_eq!(listed[0].aliases, vec!["雾儿".to_string()]);
}
