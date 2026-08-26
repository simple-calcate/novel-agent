//! 人类纠正记录与可晋升的偏好规则。
//!
//! 这一层没有 IO：仓储负责落库，应用层在拒绝续写时调用这里生成规则，
//! 并在下次续写时把规则拼进 system prompt。

use chrono::Utc;
use novel_domain::{CorrectionRecord, PreferenceRuleId, ProposalId, RejectionReason};

pub use novel_domain::{PreferenceRule, PreferenceScope, PreferenceStatus};

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

/// 把未停用的偏好规则写成续写 system prompt 前缀。
pub fn prompt_prefix(rules: &[PreferenceRule]) -> Option<String> {
    let lines: Vec<&str> = rules
        .iter()
        .filter(|rule| rule.status != PreferenceStatus::Disabled)
        .map(|rule| rule.rule.as_str())
        .collect();
    if lines.is_empty() {
        return None;
    }
    Some(format!(
        "你是网文续写助手，必须遵守给定设定。作者偏好：\n- {}",
        lines.join("\n- ")
    ))
}

fn summarize_diff(ai: &str, human: &str) -> String {
    format!(
        "AI 文本 {} 字，人类文本 {} 字",
        ai.chars().count(),
        human.chars().count()
    )
}
