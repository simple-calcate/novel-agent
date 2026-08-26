use crate::{ChapterId, EntityId, FactId, PlotThreadId, ProjectId, Revision, StoryEventId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EntityKind {
    Character,
    Location,
    Organization,
    Item,
    Ability,
    WorldRule,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanonEntity {
    pub id: EntityId,
    pub project_id: ProjectId,
    pub branch_id: String,
    pub kind: EntityKind,
    pub canonical_name: String,
    pub aliases: Vec<String>,
    pub attributes: BTreeMap<String, serde_json::Value>,
}

/// 启发式抽取得到的一条提及，尚未进入正史。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractedMention {
    pub entity_name: String,
    pub entity_kind: EntityKind,
    pub predicate: String,
    pub object: String,
    pub quote: String,
    pub confidence: f32,
}

/// 给作者审核的正史候选（或已接受/已拒绝的事实视图）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanonProposal {
    pub fact_id: FactId,
    pub entity_id: EntityId,
    pub project_id: ProjectId,
    pub chapter_id: Option<ChapterId>,
    pub entity_name: String,
    pub entity_kind: EntityKind,
    pub predicate: String,
    pub object: String,
    pub quote: String,
    pub status: FactStatus,
    pub confidence: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceRef {
    pub chapter_id: ChapterId,
    pub block_id: Option<crate::BlockId>,
    pub revision: Revision,
    pub quote: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FactStatus {
    Candidate,
    Accepted,
    Rejected,
    Superseded,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanonFact {
    pub id: FactId,
    pub entity_id: EntityId,
    pub branch_id: String,
    pub predicate: String,
    pub value: serde_json::Value,
    pub status: FactStatus,
    pub confidence: f32,
    pub source: SourceRef,
    pub valid_from: Option<StoryInstant>,
    pub valid_to: Option<StoryInstant>,
    pub revision_from: Revision,
    pub revision_to: Option<Revision>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryInstant {
    pub sequence: i64,
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Relationship {
    pub from: EntityId,
    pub to: EntityId,
    pub relation: String,
    pub branch_id: String,
    pub valid_from: Option<StoryInstant>,
    pub valid_to: Option<StoryInstant>,
    pub source: SourceRef,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryEvent {
    pub id: StoryEventId,
    pub branch_id: String,
    pub story_time: Option<StoryInstant>,
    pub narrative_order: u64,
    pub location_id: Option<EntityId>,
    pub participants: Vec<EntityId>,
    pub summary: String,
    pub causes: Vec<StoryEventId>,
    pub preconditions: Vec<FactId>,
    pub effects: Vec<StateTransition>,
    pub source: SourceRef,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StateTransition {
    pub entity_id: EntityId,
    pub key: String,
    pub before: Option<serde_json::Value>,
    pub after: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterKnowledge {
    pub character_id: EntityId,
    pub fact_id: FactId,
    pub learned_at: StoryInstant,
    pub branch_id: String,
    pub source: SourceRef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PlotThreadStatus {
    Open,
    Advanced,
    Resolved,
    Abandoned,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlotThread {
    pub id: PlotThreadId,
    pub branch_id: String,
    pub title: String,
    pub status: PlotThreadStatus,
    pub introduced_at: StoryInstant,
    pub due_by: Option<StoryInstant>,
    pub milestones: Vec<StoryEventId>,
}
