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
