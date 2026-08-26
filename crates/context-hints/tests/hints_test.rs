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
        nearby_text: "夜晚的海面".into(),
        generation: 1,
        limit: 5,
    };
    let hints = engine.rank(&query, &[], &[], &[thread]);
    assert_eq!(hints.len(), 1);
    assert_eq!(hints[0].kind, novel_domain::HintKind::OpenForeshadowing);
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
