use novel_automation::{rule_matches, stable_idempotency_key, TypingSession};
use novel_domain::{
    Actor, ConditionOperator, DomainEvent, EventSource, Platform, ProjectId, Revision,
    WorkflowCondition, WorkflowRule, WorkflowTrigger, EVENT_SCHEMA_VERSION,
};
use serde_json::json;
use uuid::Uuid;

#[test]
fn idle_detection_respects_composing() {
    let now = chrono::Utc::now();
    let mut session = TypingSession::new(now);
    session.composing = true;
    session.chars_since_commit = 100;
    session.last_input_at = now - chrono::Duration::seconds(5);
    assert!(!session.should_emit_idle(now, chrono::Duration::milliseconds(1800), 20));

    session.composing = false;
    assert!(session.should_emit_idle(now, chrono::Duration::milliseconds(1800), 20));
}

#[test]
fn paragraph_created_workflow_matches() {
    let rule = WorkflowRule {
        id: Default::default(),
        project_id: ProjectId::new(),
        name: "新段落检查".into(),
        enabled: true,
        trigger: WorkflowTrigger {
            event_type: "paragraph.created".into(),
        },
        conditions: vec![WorkflowCondition {
            path: "insertedChars".into(),
            operator: ConditionOperator::Gte,
            value: json!(100),
        }],
        actions: vec![],
        priority: 0,
        cooldown_ms: 0,
    };

    let event = DomainEvent {
        event_id: Default::default(),
        event_type: "paragraph.created".into(),
        schema_version: EVENT_SCHEMA_VERSION,
        occurred_at: chrono::Utc::now(),
        project_id: ProjectId::new(),
        book_id: None,
        chapter_id: None,
        scene_id: None,
        block_id: None,
        actor: Actor::User { user_id: None },
        source: EventSource::Editor,
        platform: Platform::Linux,
        transaction_id: "tx-1".into(),
        correlation_id: None,
        causation_id: None,
        revision_before: Revision(0),
        revision_after: Revision(1),
        payload: json!({ "insertedChars": 240, "source": "typing" }),
    };

    assert!(rule_matches(&rule, &event));
}

#[test]
fn idempotency_key_is_stable() {
    let workflow_id = Uuid::new_v4();
    let key1 = stable_idempotency_key("evt-1", &workflow_id, "save");
    let key2 = stable_idempotency_key("evt-1", &workflow_id, "save");
    assert_eq!(key1, key2);
}
