use novel_context_engine::{CompressionBlock, ContextEngine, ContextState, CoreMessage, MessageRole};
use novel_domain::SessionId;

fn make_messages(count: usize) -> Vec<CoreMessage> {
    (0..count)
        .map(|i| CoreMessage {
            id: format!("msg-{i}"),
            role: if i % 2 == 0 { MessageRole::User } else { MessageRole::Assistant },
            text: format!("这是第 {} 条消息，包含足够多的内容来模拟 token 消耗。", i),
            tokens: 20,
            protected: false,
        })
        .collect()
}

#[test]
fn nudge_triggers_when_over_budget() {
    let engine = ContextEngine {
        context_limit: 100,
        min_context_limit_pct: 0.5,
        preserve_recent_messages: 2,
    };
    let messages = make_messages(10);
    let state = ContextState {
        session_id: SessionId::new(),
        blocks: vec![],
        next_ref: 0,
    };
    let (_, _, nudge) = engine.process_turn(messages, state);
    assert!(nudge.should_inject);
    assert!(nudge.usage_ratio > 1.0);
}

#[test]
fn compression_removes_old_messages() {
    let engine = ContextEngine {
        context_limit: 1000,
        min_context_limit_pct: 0.8,
        preserve_recent_messages: 2,
    };
    let messages = make_messages(10);
    let state = ContextState {
        session_id: SessionId::new(),
        blocks: vec![],
        next_ref: 0,
    };

    let state = engine.apply_compression(
        &messages,
        state,
        "前 8 条消息的摘要".into(),
        Some("早期讨论".into()),
    );

    assert_eq!(state.blocks.len(), 1);
    assert_eq!(state.blocks[0].direct_message_ids.len(), 8);
}

#[test]
fn process_turn_replaces_with_summary() {
    let engine = ContextEngine {
        context_limit: 1000,
        min_context_limit_pct: 0.8,
        preserve_recent_messages: 2,
    };
    let messages = make_messages(10);
    let mut state = ContextState {
        session_id: SessionId::new(),
        blocks: vec![],
        next_ref: 0,
    };

    state = engine.apply_compression(
        &messages,
        state,
        "前 8 条消息的摘要".into(),
        Some("早期讨论".into()),
    );

    let (rendered, _, _) = engine.process_turn(messages, state);
    // 10 条原始消息 - 8 条被压缩 + 1 条摘要 = 3 条
    assert_eq!(rendered.len(), 3);
    assert!(rendered[0].text.contains("摘要"));
}

#[test]
fn search_finds_relevant_blocks() {
    let engine = ContextEngine {
        context_limit: 1000,
        min_context_limit_pct: 0.8,
        preserve_recent_messages: 2,
    };
    let messages = make_messages(5);
    let mut state = ContextState {
        session_id: SessionId::new(),
        blocks: vec![],
        next_ref: 0,
    };
    state = engine.apply_compression(
        &messages,
        state,
        "关于人物设定的讨论".into(),
        Some("角色".into()),
    );

    let results = engine.search(&state, "人物");
    assert_eq!(results.len(), 1);
}
