use novel_domain::{CanonFact, CharacterKnowledge, EntityId, FactStatus, StoryEvent, StoryInstant};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Default)]
pub struct StorySnapshot {
    pub entity_state: BTreeMap<EntityId, BTreeMap<String, Value>>,
    pub knowledge: BTreeMap<EntityId, Vec<String>>,
}

pub fn build_snapshot(
    facts: &[CanonFact],
    events: &[StoryEvent],
    knowledge: &[CharacterKnowledge],
    at: Option<&StoryInstant>,
) -> StorySnapshot {
    let mut snapshot = StorySnapshot::default();

    for fact in facts {
        if fact.status != FactStatus::Accepted || !fact_is_valid_at(fact, at) {
            continue;
        }
        snapshot
            .entity_state
            .entry(fact.entity_id.clone())
            .or_default()
            .insert(fact.predicate.clone(), fact.value.clone());
    }

    for event in events {
        if !event_is_visible_at(event, at) {
            continue;
        }
        for transition in &event.effects {
            snapshot
                .entity_state
                .entry(transition.entity_id.clone())
                .or_default()
                .insert(
                    transition.key.clone(),
                    transition.after.clone().unwrap_or(Value::Null),
                );
        }
    }

    for item in knowledge {
        if at.is_none_or(|instant| item.learned_at <= *instant) {
            snapshot
                .knowledge
                .entry(item.character_id.clone())
                .or_default()
                .push(item.fact_id.to_string());
        }
    }

    snapshot
}

fn fact_is_valid_at(fact: &CanonFact, at: Option<&StoryInstant>) -> bool {
    let Some(at) = at else {
        return true;
    };
    fact.valid_from.as_ref().is_none_or(|from| from <= at)
        && fact.valid_to.as_ref().is_none_or(|to| at <= to)
}

fn event_is_visible_at(event: &StoryEvent, at: Option<&StoryInstant>) -> bool {
    match (at, &event.story_time) {
        (None, _) | (_, None) => true,
        (Some(at), Some(time)) => time <= at,
    }
}
