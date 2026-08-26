use crate::{Actor, ChapterId, ProposalId, Revision, TextOperation};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentPatch {
    pub id: ProposalId,
    pub chapter_id: ChapterId,
    pub base_revision: Revision,
    pub operations: Vec<TextOperation>,
    pub rationale: String,
    pub created_by: Actor,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CorrectionRecord {
    pub proposal_id: ProposalId,
    pub ai_text: String,
    pub human_text: String,
    pub diff_summary: String,
    pub context_excerpt: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RejectionReason {
    CanonError,
    CharacterVoice,
    Pacing,
    Style,
    Fact,
    Format,
    Other,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PreferenceScope {
    Proposal,
    Character { entity_id: String },
    Project { project_id: String },
    GlobalAuthor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PreferenceStatus {
    Candidate,
    Confirmed,
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreferenceRule {
    pub id: crate::PreferenceRuleId,
    pub scope: PreferenceScope,
    pub rule: String,
    pub status: PreferenceStatus,
    pub evidence_proposals: Vec<ProposalId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
