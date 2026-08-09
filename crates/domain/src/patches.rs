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
