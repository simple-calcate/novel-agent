use novel_context_hints::{HintEngine, HintQuery};
use novel_domain::{
    CanonEntity, ChapterId, EntityId, EntityKind, PlotThread, PlotThreadStatus, ProjectId,
    Revision, StoryEntry, StoryEntryKind, StoryInstant, WorkContextRef,
};

fn work_ref() -> WorkContextRef {
    WorkContextRef {
        project_id: ProjectId::new(),
        branch_id: "main".into(),
        revision: Revision(5),
        chapter_id: ChapterId::new(),
        block_id: None,
        pov_entity_id: None,
    }
}

#[test]
fn matches_entity_by_name() {
    let engine = HintEngine {
        minimum_dwell_score: 0.0,
    };
    let entity = CanonEntity {
        id: EntityId::new(),
        project_id: ProjectId::new(),
        branch_id: "main".into(),
        kind: EntityKind::Character,
        canonical_name: "沈雾".into(),
        aliases: vec!["雾儿".into()],
        attributes: Default::default(),
    };
    let query = HintQuery {
        work_ref: work_ref(),
        nearby_text: "沈雾走进了雾港".into(),
        lookback_text: String::new(),
        generation: 1,
        limit: 5,
    };
    let hints = engine.rank(&query, &[entity], &[], &[]);
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0].title, "沈雾");
}

#[test]
fn matches_entity_by_alias() {
    let engine = HintEngine {
        minimum_dwell_score: 0.0,
    };
    let entity = CanonEntity {
        id: EntityId::new(),
        project_id: ProjectId::new(),
        branch_id: "main".into(),
        kind: EntityKind::Character,
        canonical_name: "沈雾".into(),
        aliases: vec!["雾儿".into()],
        attributes: Default::default(),
    };
    let query = HintQuery {
        work_ref: work_ref(),
        nearby_text: "雾儿没有回头".into(),
        lookback_text: String::new(),
        generation: 1,
        limit: 5,
    };
    let hints = engine.rank(&query, &[entity], &[], &[]);
    assert_eq!(hints.len(), 1);
}

#[test]
fn open_plot_thread_appears() {
    let engine = HintEngine {
        minimum_dwell_score: 0.0,
    };
    let thread = PlotThread {
        id: Default::default(),
        branch_id: "main".into(),
        title: "雾中灯塔".into(),
        summary: "灯塔里还藏着旧王玺".into(),
        status: PlotThreadStatus::Open,
        introduced_at: StoryInstant {
            sequence: 1,
            label: None,
        },
        due_by: Some(StoryInstant {
            sequence: 100,
            label: None,
        }),
        milestones: vec![],
    };
    let query = HintQuery {
        work_ref: work_ref(),
        nearby_text: "雾中灯塔在夜晚发亮".into(),
        lookback_text: String::new(),
        generation: 1,
        limit: 5,
    };
    let hints = engine.rank(&query, &[], &[], &[thread]);
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0].kind, novel_domain::HintKind::OpenForeshadowing);
}

#[test]
fn unmatched_plot_thread_stays_hidden() {
    let engine = HintEngine {
        minimum_dwell_score: 0.0,
    };
    let thread = PlotThread {
        id: Default::default(),
        branch_id: "main".into(),
        title: "雾中灯塔".into(),
        summary: String::new(),
        status: PlotThreadStatus::Open,
        introduced_at: StoryInstant {
            sequence: 1,
            label: None,
        },
        due_by: None,
        milestones: vec![],
    };
    let query = HintQuery {
        work_ref: work_ref(),
        nearby_text: "夜晚的海面".into(),
        lookback_text: String::new(),
        generation: 1,
        limit: 5,
    };
    let hints = engine.rank(&query, &[], &[], &[thread]);
    assert!(hints.is_empty());
}

#[test]
fn result_limit_respected() {
    let engine = HintEngine {
        minimum_dwell_score: 0.0,
    };
    let entities: Vec<CanonEntity> = (0..10)
        .map(|i| CanonEntity {
            id: EntityId::new(),
            project_id: ProjectId::new(),
            branch_id: "main".into(),
            kind: EntityKind::Character,
            canonical_name: format!("角色{i}"),
            aliases: vec![],
            attributes: Default::default(),
        })
        .collect();
    let query = HintQuery {
        work_ref: work_ref(),
        nearby_text: "角色0 角色1 角色2 角色3 角色4".into(),
        lookback_text: String::new(),
        generation: 1,
        limit: 3,
    };
    let hints = engine.rank(&query, &entities, &[], &[]);
    assert_eq!(hints.len(), 3);
}

#[test]
fn designed_entries_match_current_paragraph_and_keep_summary() {
    let engine = HintEngine {
        minimum_dwell_score: 0.0,
    };
    let project_id = ProjectId::new();
    let entries = vec![
        novel_domain::StoryEntry {
            id: "1".into(),
            project_id: project_id.clone(),
            kind: novel_domain::StoryEntryKind::Character,
            title: "林晚".into(),
            summary: "雾港来的刀客".into(),
            aliases: vec![],
        },
        novel_domain::StoryEntry {
            id: "2".into(),
            project_id: project_id.clone(),
            kind: novel_domain::StoryEntryKind::Foreshadow,
            title: "雾中灯塔".into(),
            summary: "里面还有旧王玺".into(),
            aliases: vec![],
        },
        novel_domain::StoryEntry {
            id: "3".into(),
            project_id,
            kind: novel_domain::StoryEntryKind::Setting,
            title: "雾港".into(),
            summary: "终年被海雾罩住".into(),
            aliases: vec![],
        },
    ];
    let query = HintQuery {
        work_ref: work_ref(),
        nearby_text: "林晚走进雾港".into(),
        lookback_text: String::new(),
        generation: 1,
        limit: 5,
    };
    let hints = engine.rank_entries(&query, &entries);
    assert_eq!(hints.len(), 2);
    assert_eq!(hints[0].title, "林晚");
    assert_eq!(hints[0].summary, "雾港来的刀客");
    assert_eq!(hints[1].title, "雾港");
    assert!(hints.iter().all(|hint| hint.title != "雾中灯塔"));
}

#[test]
fn unmatched_designed_entry_stays_hidden() {
    let engine = HintEngine {
        minimum_dwell_score: 0.0,
    };
    let entry = novel_domain::StoryEntry {
        id: "1".into(),
        project_id: ProjectId::new(),
        kind: novel_domain::StoryEntryKind::Character,
        title: "林晚".into(),
        summary: "雾港来的刀客".into(),
        aliases: vec![],
    };
    let query = HintQuery {
        work_ref: work_ref(),
        nearby_text: "夜晚的海面".into(),
        lookback_text: String::new(),
        generation: 1,
        limit: 5,
    };
    let hints = engine.rank_entries(&query, &[entry]);
    assert!(hints.is_empty());
}

fn lin_wan() -> novel_domain::StoryEntry {
    novel_domain::StoryEntry {
        id: "1".into(),
        project_id: ProjectId::new(),
        kind: novel_domain::StoryEntryKind::Character,
        title: "林晚".into(),
        summary: "雾港来的刀客".into(),
        aliases: vec!["雾儿".into()],
    }
}

fn lighthouse() -> novel_domain::StoryEntry {
    novel_domain::StoryEntry {
        id: "2".into(),
        project_id: ProjectId::new(),
        kind: novel_domain::StoryEntryKind::Foreshadow,
        title: "雾中灯塔".into(),
        summary: "里面还有旧王玺".into(),
        aliases: vec![],
    }
}

fn rank_one(
    nearby: &str,
    lookback: &str,
    entry: novel_domain::StoryEntry,
) -> Vec<novel_domain::ContextHint> {
    HintEngine {
        minimum_dwell_score: 0.2,
    }
    .rank_entries(
        &HintQuery {
            work_ref: work_ref(),
            nearby_text: nearby.into(),
            lookback_text: lookback.into(),
            generation: 1,
            limit: 5,
        },
        &[entry],
    )
}

#[test]
fn matches_alias_in_current_paragraph() {
    let hints = rank_one("雾儿没有回头", "", lin_wan());
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0].title, "林晚");
    assert!(
        hints[0].match_reason.contains("雾儿"),
        "{}",
        hints[0].match_reason
    );
}

#[test]
fn matches_summary_keyword() {
    let hints = rank_one("那个刀客转过身", "", lin_wan());
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0].title, "林晚");
    assert!(
        hints[0].match_reason.contains("刀客"),
        "{}",
        hints[0].match_reason
    );
}

#[test]
fn matches_title_core_for_long_name() {
    let hints = rank_one("那座灯塔夜里忽然亮了", "", lighthouse());
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0].title, "雾中灯塔");
}

#[test]
fn matches_summary_object() {
    let hints = rank_one("旧王玺还在匣中", "", lighthouse());
    assert_eq!(hints.len(), 1);
    assert!(
        hints[0].match_reason.contains("旧王玺"),
        "{}",
        hints[0].match_reason
    );
}

#[test]
fn previous_paragraph_keeps_character() {
    let hints = rank_one("她没有回头", "林晚走进雾港", lin_wan());
    assert_eq!(hints.len(), 1);
    assert!(
        hints[0].match_reason.contains("上一段"),
        "{}",
        hints[0].match_reason
    );
}

#[test]
fn unrelated_paragraph_without_lookback_stays_empty() {
    let hints = rank_one("夜晚的海面", "", lin_wan());
    assert!(hints.is_empty());
}

#[test]
fn lexical_retrieval_matches_summary_term_missing_from_local_keywords() {
    let entry = StoryEntry {
        id: "4".into(),
        project_id: ProjectId::new(),
        kind: StoryEntryKind::Character,
        title: "灯塔守夜人".into(),
        summary: "负责在雾季敲钟".into(),
        aliases: vec![],
    };
    let hints = rank_one("雾季快到了", "", entry);
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0].title, "灯塔守夜人");
    assert!(
        hints[0].match_reason.contains("检索到"),
        "{}",
        hints[0].match_reason
    );
    assert!(
        hints[0].match_reason.contains("雾季"),
        "{}",
        hints[0].match_reason
    );
}

fn fixture_entries(project_id: &ProjectId) -> Vec<StoryEntry> {
    vec![
        StoryEntry {
            id: "1".into(),
            project_id: project_id.clone(),
            kind: StoryEntryKind::Character,
            title: "林晚".into(),
            summary: "雾港来的刀客".into(),
            aliases: vec!["雾儿".into()],
        },
        StoryEntry {
            id: "2".into(),
            project_id: project_id.clone(),
            kind: StoryEntryKind::Foreshadow,
            title: "雾中灯塔".into(),
            summary: "里面还有旧王玺".into(),
            aliases: vec![],
        },
        StoryEntry {
            id: "3".into(),
            project_id: project_id.clone(),
            kind: StoryEntryKind::Setting,
            title: "雾港".into(),
            summary: "终年被海雾罩住".into(),
            aliases: vec![],
        },
    ]
}

fn extra_fixture_entries(project_id: &ProjectId, extra: &serde_json::Value) -> Vec<StoryEntry> {
    extra
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| {
            Some(StoryEntry {
                id: item.get("id")?.as_str()?.to_owned(),
                project_id: project_id.clone(),
                kind: match item.get("kind")?.as_str()? {
                    "setting" => StoryEntryKind::Setting,
                    "foreshadow" => StoryEntryKind::Foreshadow,
                    _ => StoryEntryKind::Character,
                },
                title: item.get("title")?.as_str()?.to_owned(),
                summary: item
                    .get("summary")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                aliases: item
                    .get("aliases")
                    .and_then(serde_json::Value::as_array)
                    .map(|aliases| {
                        aliases
                            .iter()
                            .filter_map(|value| value.as_str().map(str::to_owned))
                            .collect()
                    })
                    .unwrap_or_default(),
            })
        })
        .collect()
}

#[test]
fn shared_match_fixtures_agree_with_typescript() {
    let raw = include_str!("../../../packages/match-fixtures/cases.json");
    let cases: Vec<serde_json::Value> = serde_json::from_str(raw).unwrap();
    let project_id = ProjectId::new();
    let engine = HintEngine {
        minimum_dwell_score: 0.0,
    };
    for case in cases {
        let id = case["id"].as_str().unwrap();
        let current = case["current"].as_str().unwrap();
        let lookback = case["lookback"].as_str().unwrap();
        let expected: Vec<String> = case["expectedTitles"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|value| value.as_str().map(str::to_owned))
            .collect();
        let mut entries = fixture_entries(&project_id);
        entries.extend(extra_fixture_entries(&project_id, &case["extraEntries"]));
        let hints = engine.rank_entries(
            &HintQuery {
                work_ref: work_ref(),
                nearby_text: current.into(),
                lookback_text: lookback.into(),
                generation: 1,
                limit: 5,
            },
            &entries,
        );
        let titles: Vec<String> = hints.iter().map(|hint| hint.title.clone()).collect();
        assert_eq!(titles, expected, "case {id}");
        assert!(hints
            .iter()
            .all(|hint| entries.iter().any(|entry| entry.id == hint.id)));
    }
}
