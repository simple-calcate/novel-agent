use novel_context_hints::{HintEngine, HintQuery};
use novel_domain::{
    CanonEntity, ChapterId, EntityId, EntityKind, PlotThread, PlotThreadStatus, ProjectId,
    Revision, StoryInstant, WorkContextRef,
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
