use novel_domain::{ProposalId, RejectionReason};
use novel_feedback_memory::{
    correction_from_edit, rejection_rule, PreferenceScope, PreferenceStatus,
};

#[test]
fn no_correction_when_text_unchanged() {
    let result = correction_from_edit(ProposalId::new(), "原文", "原文", "上下文");
    assert!(result.is_none());
}

#[test]
fn correction_when_text_changed() {
    let result = correction_from_edit(
        ProposalId::new(),
        "AI 生成的文字",
        "人类修改后的文字",
        "上下文",
    );
    assert!(result.is_some());
    let record = result.unwrap();
    assert!(record.ai_text.contains("AI"));
    assert!(record.human_text.contains("人类"));
}

#[test]
fn rejection_creates_candidate_rule() {
    let rule = rejection_rule(
        RejectionReason::CharacterVoice,
        PreferenceScope::GlobalAuthor,
        ProposalId::new(),
    );
    assert_eq!(rule.status, PreferenceStatus::Candidate);
    assert!(rule.rule.contains("口吻"));
}
