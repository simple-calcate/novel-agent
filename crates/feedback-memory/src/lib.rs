//! 人类纠正记录与可晋升的偏好规则。

use chrono::Utc;
use novel_domain::{CorrectionRecord, PreferenceRuleId, ProposalId, RejectionReason};
use serde::{Deserialize, Serialize};

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
    pub id: PreferenceRuleId,
    pub scope: PreferenceScope,
    pub rule: String,
    pub status: PreferenceStatus,
    pub evidence_proposals: Vec<ProposalId>,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

pub fn correction_from_edit(
    proposal_id: ProposalId,
    ai_text: &str,
    human_text: &str,
    context_excerpt: &str,
) -> Option<CorrectionRecord> {
    if ai_text == human_text {
        return None;
    }
    Some(CorrectionRecord {
        proposal_id,
        ai_text: ai_text.into(),
        human_text: human_text.into(),
        diff_summary: summarize_diff(ai_text, human_text),
        context_excerpt: context_excerpt.into(),
        created_at: Utc::now(),
    })
}

pub fn rejection_rule(
    reason: RejectionReason,
    scope: PreferenceScope,
    evidence: ProposalId,
) -> PreferenceRule {
    let now = Utc::now();
    PreferenceRule {
        id: PreferenceRuleId::new(),
        scope,
        rule: match reason {
            RejectionReason::CanonError => "避免违反正史设定",
            RejectionReason::CharacterVoice => "保持人物口吻一致",
            RejectionReason::Pacing => "避免节奏过快或过慢",
            RejectionReason::Style => "遵循作者文风",
            RejectionReason::Fact => "避免事实错误",
            RejectionReason::Format => "遵循指定输出格式",
            RejectionReason::Other => "尊重作者明确拒绝",
        }
        .into(),
        status: PreferenceStatus::Candidate,
        evidence_proposals: vec![evidence],
        created_at: now,
        updated_at: now,
    }
}

fn summarize_diff(ai: &str, human: &str) -> String {
    format!(
        "AI 文本 {} 字，人类文本 {} 字",
        ai.chars().count(),
        human.chars().count()
    )
}
