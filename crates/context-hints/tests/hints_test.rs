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
        },
        novel_domain::StoryEntry {
            id: "2".into(),
            project_id: project_id.clone(),
            kind: novel_domain::StoryEntryKind::Foreshadow,
            title: "雾中灯塔".into(),
            summary: "里面还有旧王玺".into(),
        },
        novel_domain::StoryEntry {
            id: "3".into(),
            project_id,
            kind: novel_domain::StoryEntryKind::Setting,
            title: "雾港".into(),
            summary: "终年被海雾罩住".into(),
        },
    ];
    let query = HintQuery {
        work_ref: work_ref(),
        nearby_text: "林晚走进雾港".into(),
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
    };
    let query = HintQuery {
        work_ref: work_ref(),
        nearby_text: "夜晚的海面".into(),
        generation: 1,
        limit: 5,
    };
    let hints = engine.rank_entries(&query, &[entry]);
    assert!(hints.is_empty());
}
