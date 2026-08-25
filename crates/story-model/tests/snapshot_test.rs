use novel_domain::{
    CanonFact, EntityId, FactId, FactStatus, Revision, SourceRef, StateTransition, StoryEvent,
    StoryEventId, StoryInstant,
};
use novel_story_model::{build_snapshot, validate_at, IssueSeverity};
use serde_json::json;

fn make_fact(entity_id: EntityId, predicate: &str, value: serde_json::Value) -> CanonFact {
    CanonFact {
        id: FactId::new(),
        entity_id,
        branch_id: "main".into(),
        predicate: predicate.into(),
        value,
        status: FactStatus::Accepted,
        confidence: 1.0,
        source: SourceRef {
            chapter_id: Default::default(),
            block_id: None,
            revision: Revision(0),
            quote: String::new(),
        },
        valid_from: None,
        valid_to: None,
        revision_from: Revision(0),
        revision_to: None,
    }
}

#[test]
fn snapshot_applies_state_transitions() {
    let entity = EntityId::new();
    let facts = vec![make_fact(entity.clone(), "alive", json!(true))];
    let events = vec![StoryEvent {
        id: StoryEventId::new(),
        branch_id: "main".into(),
        story_time: Some(StoryInstant {
            sequence: 10,
            label: None,
        }),
        narrative_order: 1,
        location_id: None,
        participants: vec![entity.clone()],
        summary: "角色死亡".into(),
        causes: vec![],
        preconditions: vec![],
        effects: vec![StateTransition {
            entity_id: entity.clone(),
            key: "alive".into(),
            before: Some(json!(true)),
            after: Some(json!(false)),
        }],
        source: SourceRef {
            chapter_id: Default::default(),
            block_id: None,
            revision: Revision(0),
            quote: String::new(),
        },
        created_at: chrono::Utc::now(),
    }];

    let snapshot = build_snapshot(
        &facts,
        &events,
        &[],
        Some(&StoryInstant {
            sequence: 5,
            label: None,
        }),
    );
    assert_eq!(
        snapshot
            .entity_state
            .get(&entity)
            .and_then(|s| s.get("alive")),
        Some(&json!(true))
    );

    let snapshot = build_snapshot(
        &facts,
        &events,
        &[],
        Some(&StoryInstant {
            sequence: 15,
            label: None,
        }),
    );
    assert_eq!(
        snapshot
            .entity_state
            .get(&entity)
            .and_then(|s| s.get("alive")),
        Some(&json!(false))
    );
}

#[test]
fn validate_dead_character_appears_later() {
    let entity = EntityId::new();
    let facts = vec![make_fact(entity.clone(), "alive", json!(true))];
    let events = vec![
        StoryEvent {
            id: StoryEventId::new(),
            branch_id: "main".into(),
            story_time: Some(StoryInstant {
                sequence: 10,
                label: None,
            }),
            narrative_order: 1,
            location_id: None,
            participants: vec![entity.clone()],
            summary: "死亡".into(),
            causes: vec![],
            preconditions: vec![],
            effects: vec![StateTransition {
                entity_id: entity.clone(),
                key: "alive".into(),
                before: Some(json!(true)),
                after: Some(json!(false)),
            }],
            source: SourceRef {
                chapter_id: Default::default(),
                block_id: None,
                revision: Revision(0),
                quote: String::new(),
            },
            created_at: chrono::Utc::now(),
        },
        StoryEvent {
            id: StoryEventId::new(),
            branch_id: "main".into(),
            story_time: Some(StoryInstant {
                sequence: 20,
                label: None,
            }),
            narrative_order: 2,
            location_id: None,
            participants: vec![entity.clone()],
            summary: "再次登场".into(),
            causes: vec![],
            preconditions: vec![],
            effects: vec![],
            source: SourceRef {
                chapter_id: Default::default(),
                block_id: None,
                revision: Revision(0),
                quote: String::new(),
            },
            created_at: chrono::Utc::now(),
        },
    ];

    let issues = validate_at(
        &facts,
        &events,
        &[],
        &StoryInstant {
            sequence: 15,
            label: None,
        },
    );
    assert!(!issues.is_empty());
    assert!(issues.iter().any(|i| i.code == "dead-character-active"));
    assert!(issues.iter().any(|i| i.severity == IssueSeverity::Error));
}
