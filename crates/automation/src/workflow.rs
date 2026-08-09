use novel_domain::{ConditionOperator, DomainEvent, WorkflowCondition, WorkflowRule};
use serde_json::Value;

pub fn rule_matches(rule: &WorkflowRule, event: &DomainEvent) -> bool {
    rule.enabled
        && rule.trigger.event_type == event.event_type
        && rule
            .conditions
            .iter()
            .all(|condition| condition_matches(condition, event))
}

fn condition_matches(condition: &WorkflowCondition, event: &DomainEvent) -> bool {
    let Some(value) = lookup_path(&event.payload, &condition.path) else {
        return condition.operator == ConditionOperator::Exists
            && condition.value == Value::Bool(false);
    };

    match condition.operator {
        ConditionOperator::Eq => value == &condition.value,
        ConditionOperator::NotEq => value != &condition.value,
        ConditionOperator::Gt => compare_numbers(value, &condition.value, |left, right| left > right),
        ConditionOperator::Gte => compare_numbers(value, &condition.value, |left, right| left >= right),
        ConditionOperator::Lt => compare_numbers(value, &condition.value, |left, right| left < right),
        ConditionOperator::Lte => compare_numbers(value, &condition.value, |left, right| left <= right),
        ConditionOperator::Contains => match (value, &condition.value) {
            (Value::String(left), Value::String(right)) => left.contains(right),
            (Value::Array(left), right) => left.contains(right),
            _ => false,
        },
        ConditionOperator::Exists => condition.value == Value::Bool(true),
    }
}

fn lookup_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = value;
    for segment in path.split('.') {
        current = current.get(segment)?;
    }
    Some(current)
}

fn compare_numbers(
    left: &Value,
    right: &Value,
    predicate: impl FnOnce(f64, f64) -> bool,
) -> bool {
    match (left.as_f64(), right.as_f64()) {
        (Some(left), Some(right)) => predicate(left, right),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use novel_domain::{Actor, EventSource, Platform, ProjectId, Revision, EVENT_SCHEMA_VERSION};
    use serde_json::json;

    #[test]
    fn matches_numeric_condition() {
        let rule = WorkflowRule {
            id: Default::default(),
            project_id: ProjectId::new(),
            name: "长段落后检查".into(),
            enabled: true,
            trigger: novel_domain::WorkflowTrigger {
                event_type: "paragraph.created".into(),
            },
            conditions: vec![WorkflowCondition {
                path: "insertedChars".into(),
                operator: ConditionOperator::Gte,
                value: json!(200),
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
            transaction_id: "tx".into(),
            correlation_id: None,
            causation_id: None,
            revision_before: Revision(1),
            revision_after: Revision(2),
            payload: json!({ "insertedChars": 240 }),
        };
        assert!(rule_matches(&rule, &event));
    }
}
