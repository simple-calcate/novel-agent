use novel_story_model::extract_mentions;

#[test]
fn extracts_speakers_titles_and_locations() {
    let text = "林晚说道：「今夜雾很重。」沈雾问道：「你要去雾港吗？」两人走进雾港码头。《潮汐秘录》就放在案上。";
    let mentions = extract_mentions(text);
    let names: Vec<_> = mentions
        .iter()
        .map(|mention| (mention.entity_kind.clone(), mention.entity_name.as_str()))
        .collect();
    assert!(
        names.contains(&(novel_domain::EntityKind::Character, "林晚")),
        "{names:?}"
    );
    assert!(names.contains(&(novel_domain::EntityKind::Character, "沈雾")));
    assert!(names.contains(&(novel_domain::EntityKind::Location, "雾港码头")));
    assert!(names.contains(&(novel_domain::EntityKind::Item, "潮汐秘录")));
}

#[test]
fn ignores_stopwords_and_dedupes() {
    let text = "然后说道：「嗯。」林晚说道：「再来。」林晚说道：「再来。」";
    let mentions = extract_mentions(text);
    let speakers: Vec<_> = mentions
        .iter()
        .filter(|mention| mention.predicate == "appearsAsSpeaker")
        .map(|mention| mention.entity_name.as_str())
        .collect();
    assert_eq!(speakers, vec!["林晚"]);
}
