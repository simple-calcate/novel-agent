use novel_agent_runtime::{AgentBudget, AgentRun, AgentRuntime, EchoProvider};
use novel_domain::{ChapterId, Revision};

#[tokio::test]
async fn echo_provider_generates_patch() {
    let runtime = AgentRuntime::new(EchoProvider);
    let run = AgentRun {
        id: Default::default(),
        chapter_id: ChapterId::new(),
        base_revision: Revision(3),
        prompt: "续写这一段".into(),
        budget: AgentBudget {
            max_rounds: 1,
            max_tokens: 512,
            max_cost_micros: 0,
            max_seconds: 30,
        },
        started_at: chrono::Utc::now(),
    };

    let patch = runtime
        .run_continuation(run, "雾港的夜晚".into())
        .await
        .unwrap();
    assert_eq!(patch.base_revision, Revision(3));
    assert!(!patch.operations.is_empty());
    assert!(patch.rationale.contains("续写"));
}
