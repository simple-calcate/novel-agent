use crate::{ChapterId, EntityId, ProjectId, Revision};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkContextRef {
    pub project_id: ProjectId,
    pub branch_id: String,
    pub revision: Revision,
    pub chapter_id: ChapterId,
    pub block_id: Option<crate::BlockId>,
    pub pov_entity_id: Option<EntityId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextSource {
    pub label: String,
    pub chapter_id: Option<ChapterId>,
    pub revision: Option<Revision>,
    pub confidence: f32,
    pub token_cost: u32,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextPackage {
    pub id: String,
    pub work_ref: WorkContextRef,
    pub sections: Vec<ContextSection>,
    pub sources: Vec<ContextSource>,
    pub token_budget: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextSection {
    pub title: String,
    pub text: String,
    pub priority: u32,
    pub required: bool,
    pub source: ContextSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HintKind {
    CharacterState,
    WorldRule,
    TimelineConstraint,
    OpenForeshadowing,
    PlotHook,
    Preference,
    ContinuityRisk,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextHint {
    pub id: String,
    pub kind: HintKind,
    pub title: String,
    pub summary: String,
    pub source_label: String,
    pub match_reason: String,
    pub confidence: f32,
    pub score: f32,
    pub generation: u64,
    pub revision: Revision,
    pub actions: Vec<HintAction>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HintAction {
    ExpandSource,
    Pin,
    Snooze,
    Ignore,
    MarkWrong,
}
