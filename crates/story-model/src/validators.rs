use crate::{build_snapshot, StorySnapshot};
use novel_domain::{CanonFact, CharacterKnowledge, StoryEvent, StoryInstant};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinuityIssue {
    pub severity: IssueSeverity,
    pub code: String,
    pub message: String,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum IssueSeverity {
    Info,
    Warning,
    Error,
}

pub fn validate_at(
    facts: &[CanonFact],
    events: &[StoryEvent],
    knowledge: &[CharacterKnowledge],
    at: &StoryInstant,
) -> Vec<ContinuityIssue> {
    let snapshot = build_snapshot(facts, events, knowledge, Some(at));
    let mut issues = Vec::new();
    validate_dead_entities(&snapshot, events, at, &mut issues);
    validate_mutually_exclusive_locations(&snapshot, &mut issues);
    issues
}

fn validate_dead_entities(
    snapshot: &StorySnapshot,
    events: &[StoryEvent],
    at: &StoryInstant,
    issues: &mut Vec<ContinuityIssue>,
) {
    for (entity_id, state) in &snapshot.entity_state {
        if state.get("alive") != Some(&Value::Bool(false)) {
            continue;
        }
        let active_again = events.iter().any(|event| {
            event.participants.contains(entity_id)
                && event
                    .story_time
                    .as_ref()
                    .is_some_and(|time| time > at)
        });
        if active_again {
            issues.push(ContinuityIssue {
                severity: IssueSeverity::Error,
                code: "dead-character-active".into(),
                message: "人物在当前时点已死亡，但后续事件中再次出现".into(),
                evidence: vec![entity_id.to_string()],
            });
        }
    }
}

fn validate_mutually_exclusive_locations(
    snapshot: &StorySnapshot,
    issues: &mut Vec<ContinuityIssue>,
) {
    for (entity_id, state) in &snapshot.entity_state {
        if let Some(Value::Array(locations)) = state.get("locations") {
            if locations.len() > 1 {
                issues.push(ContinuityIssue {
                    severity: IssueSeverity::Error,
                    code: "multiple-locations".into(),
                    message: "同一人物在同一时点存在多个位置".into(),
                    evidence: vec![entity_id.to_string()],
                });
            }
        }
    }
}
