//! Tauri 命令层测试：用 tauri::test 的 mock 应用直接调用命令函数，
//! 覆盖配置回退链、事件 → 工作流 → 队列全链路、参数校验与内核自描述。

use crate::{
    build_context_package, context_hints, create_book, create_chapter, create_project, delete_book,
    delete_chapter, editor_tick, emit_domain_event, generate_continuation, install_plugin_manifest,
    kernel_tools, load_chapter, load_library, load_model_config, move_book, rename_book,
    rename_chapter, rename_project, run_queue_step, save_chapter, save_model_config, AppState,
    EditorTickInput, HintRequest, ModelConfigInput, NewBookInput, NewChapterInput, NewProjectInput,
};
use novel_domain::{
    Actor, BlockKind, ContentBlock, DomainEvent, EventId, EventSource, Platform, Revision,
    WorkflowAction, WorkflowRule, WorkflowTrigger, EVENT_SCHEMA_VERSION,
};
use novel_extensions::BuiltinsExtension;
use novel_kernel::Kernel;
use novel_storage::StorageHandle;
use serde_json::json;
use std::sync::{Arc, Mutex};
use tauri::test::{mock_app, MockRuntime};
use tauri::Manager;

fn mock_app_with_kernel() -> tauri::App<MockRuntime> {
    let storage = Arc::new(StorageHandle::open_in_memory().unwrap());
    let kernel = Kernel::builder()
        .service(storage)
        .extension(BuiltinsExtension)
        .expect("内置扩展注册失败")
        .build()
        .unwrap();
    let app = mock_app();
    app.manage(AppState {
        kernel: Arc::new(kernel),
        typing_session: Mutex::new(novel_automation::TypingSession::new(chrono::Utc::now())),
    });
    app
}

fn storage_of(app: &tauri::App<MockRuntime>) -> Arc<StorageHandle> {
    app.handle()
        .state::<AppState>()
        .kernel
        .service::<StorageHandle>()
        .unwrap()
}

#[test]
fn create_project_and_chapter_roundtrip() {
    let app = mock_app_with_kernel();
    let state = || app.state::<AppState>();

    let project = create_project(
        state(),
        NewProjectInput {
            title: "夜航星图".into(),
        },
    );
    assert!(project.ok, "{project:?}");
    let project_id = project.data.unwrap().id.to_string();

    let book = create_book(
        state(),
        NewBookInput {
            project_id: project_id.clone(),
            title: "卷一".into(),
            synopsis: String::new(),
            position: 0,
        },
    );
    assert!(book.ok, "{book:?}");
    let book = book.data.unwrap();

    let chapter = create_chapter(
        state(),
        NewChapterInput {
            project_id: project_id.clone(),
            book_id: book.id.to_string(),
            title: "第一章".into(),
            position: 1,
        },
    );
    assert!(chapter.ok, "{chapter:?}");

    // 非法 project id 被拒绝
    let bad = create_chapter(
        state(),
        NewChapterInput {
            project_id: "not-a-uuid".into(),
            book_id: book.id.to_string(),
            title: "x".into(),
            position: 2,
        },
    );
    assert!(!bad.ok);
    assert!(bad.error.unwrap().contains("invalid project id"));

    let library = load_library(state(), Some(project_id.clone()));
    assert!(library.ok, "{library:?}");
    let snapshot = library.data.unwrap();
    assert_eq!(snapshot.books.len(), 1);
    assert_eq!(snapshot.chapters.len(), 1);

    let chapter_id = snapshot.chapters[0].id.to_string();
    let saved = save_chapter(state(), chapter_id.clone(), "雾港来客。".into(), None);
    assert!(saved.ok, "{saved:?}");
    let loaded = load_chapter(state(), chapter_id.clone());
    assert_eq!(loaded.data.unwrap().text, "雾港来客。");

    let blocks = vec![
        ContentBlock {
            id: novel_domain::BlockId::new(),
            kind: BlockKind::Thinking,
            text: "先写雾".into(),
            position: 0,
            markup: vec![],
        },
        ContentBlock {
            id: novel_domain::BlockId::new(),
            kind: BlockKind::Body,
            text: "雾港来客。".into(),
            position: 1,
            markup: vec![],
        },
    ];
    let saved_blocks = save_chapter(
        state(),
        chapter_id.clone(),
        "雾港来客。".into(),
        Some(blocks),
    );
    assert!(saved_blocks.ok, "{saved_blocks:?}");
    let with_blocks = load_chapter(state(), chapter_id.clone());
    assert_eq!(with_blocks.data.as_ref().unwrap().blocks.len(), 2);
    assert_eq!(
        with_blocks.data.unwrap().blocks[0].kind,
        BlockKind::Thinking
    );

    let extra_book = create_book(
        state(),
        NewBookInput {
            project_id: project_id.clone(),
            title: "卷二".into(),
            synopsis: String::new(),
            position: 0,
        },
    );
    assert!(extra_book.ok);
    let extra_id = extra_book.data.unwrap().id.to_string();
    let moved = move_book(state(), project_id.clone(), extra_id.clone(), -1);
    assert_eq!(moved.data.unwrap().books[0].title, "卷二");

    let renamed = rename_chapter(
        state(),
        project_id.clone(),
        chapter_id.clone(),
        "序章".into(),
    );
    assert_eq!(renamed.data.unwrap().chapters[0].title, "序章");
    let _ = rename_book(
        state(),
        project_id.clone(),
        extra_id.clone(),
        "卷二 · 改".into(),
    );
    let after_delete_chapter = delete_chapter(state(), project_id.clone(), chapter_id);
    assert!(after_delete_chapter.data.unwrap().chapters.is_empty());
    let after_delete_book = delete_book(state(), project_id.clone(), extra_id);
    assert_eq!(after_delete_book.data.unwrap().books.len(), 1);
    let renamed_project = rename_project(state(), project_id.clone(), "改名".into());
    assert_eq!(renamed_project.data.unwrap().projects[0].title, "改名");
}

#[tokio::test]
async fn model_config_roundtrip() {
    let app = mock_app_with_kernel();
    let state = || app.state::<AppState>();

    assert_eq!(load_model_config(state()).unwrap(), json!(null));

    let config = ModelConfigInput {
        provider: "deepseek".into(),
        api_key: "sk-test".into(),
        base_url: "https://api.deepseek.com".into(),
        model: "deepseek-chat".into(),
    };
    save_model_config(state(), config).await.unwrap();

    let loaded = load_model_config(state()).unwrap();
    assert_eq!(loaded["provider"], "deepseek");
    assert_eq!(loaded["model"], "deepseek-chat");
}

#[tokio::test]
async fn generate_continuation_falls_back_to_echo() {
    let app = mock_app_with_kernel();
    let state = || app.state::<AppState>();

    // 未配置模型 → echo 回退，返回可用补丁而不是报错
    let patch = generate_continuation(
        state(),
        "c1".into(),
        0,
        "继续当前剧情".into(),
        "雾港、旧王玺".into(),
        None,
    )
    .await
    .unwrap();
    let text = match &patch.operations[0] {
        novel_domain::TextOperation::Insert { text, .. } => text.clone(),
        _ => unreachable!(),
    };
    assert!(text.contains("请配置真实模型提供方"), "{text}");
    assert_eq!(patch.base_revision, Revision(0));
}

#[tokio::test]
async fn event_workflow_queue_end_to_end() {
    let app = mock_app_with_kernel();
    let state = || app.state::<AppState>();

    let project = create_project(
        state(),
        NewProjectInput {
            title: "流程测试".into(),
        },
    )
    .data
    .unwrap();
    let project_id = project.id.clone();

    let rule = WorkflowRule {
        id: Default::default(),
        project_id: project_id.clone(),
        name: "停笔保存".into(),
        enabled: true,
        trigger: WorkflowTrigger {
            event_type: "editor.idle".into(),
        },
        conditions: vec![],
        actions: vec![WorkflowAction::SaveDocument],
        priority: 100,
        cooldown_ms: 0,
    };
    storage_of(&app)
        .execute(|repository| repository.save_workflow(&rule))
        .unwrap();

    let event = DomainEvent {
        event_id: EventId::new(),
        event_type: "editor.idle".into(),
        schema_version: EVENT_SCHEMA_VERSION,
        occurred_at: chrono::Utc::now(),
        project_id,
        book_id: None,
        chapter_id: None,
        scene_id: None,
        block_id: None,
        actor: Actor::User { user_id: None },
        source: EventSource::Editor,
        platform: Platform::Linux,
        transaction_id: format!(
            "tx-{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ),
        correlation_id: None,
        causation_id: None,
        revision_before: Revision(0),
        revision_after: Revision(1),
        payload: json!({ "idleMs": 2000, "composing": false }),
    };
    let result = emit_domain_event(app.handle().clone(), state(), event);
    assert!(result.ok, "{result:?}");
    assert_eq!(result.data.unwrap()["queued"], 1);

    // 队列驱动一步执行
    let outcome = run_queue_step(app.handle().clone(), state()).await.unwrap();
    assert!(outcome.ok, "{outcome:?}");
    let data = outcome.data.unwrap();
    assert_eq!(data["executed"], true);
    assert_eq!(data["success"], true);
    assert_eq!(data["operation"], "document.save");

    // 再跑一步：队列已空
    let outcome = run_queue_step(app.handle().clone(), state()).await.unwrap();
    assert_eq!(outcome.data.unwrap()["executed"], false);
}

#[tokio::test]
async fn context_hints_rejects_invalid_project() {
    let app = mock_app_with_kernel();
    let state = || app.state::<AppState>();

    let result = context_hints(
        state(),
        HintRequest {
            project_id: "not-a-uuid".into(),
            chapter_id: "c1".into(),
            revision: 3,
            nearby_text: "雾港".into(),
            generation: 1,
        },
    )
    .await
    .unwrap();
    assert!(!result.ok);
    assert!(result.error.unwrap().contains("invalid project id"));
}

#[tokio::test]
async fn context_hints_accepts_valid_project() {
    let app = mock_app_with_kernel();
    let state = || app.state::<AppState>();
    let project = create_project(
        state(),
        NewProjectInput {
            title: "提示测试".into(),
        },
    )
    .data
    .unwrap();

    let result = context_hints(
        state(),
        HintRequest {
            project_id: project.id.to_string(),
            chapter_id: "c1".into(),
            revision: 3,
            nearby_text: "雾港".into(),
            generation: 1,
        },
    )
    .await
    .unwrap();
    assert!(result.ok, "{result:?}");
    assert_eq!(result.data.unwrap(), json!([]));
}

#[tokio::test]
async fn build_context_package_assembles_sections() {
    let app = mock_app_with_kernel();
    let state = || app.state::<AppState>();
    let project = create_project(
        state(),
        NewProjectInput {
            title: "上下文测试".into(),
        },
    )
    .data
    .unwrap();

    let result = build_context_package(
        state(),
        project.id.to_string(),
        "c1".into(),
        2,
        "写一段夜戏".into(),
        "雾港码头".into(),
        vec!["沈雾怕火".into()],
        vec!["潮下城禁令".into()],
        vec![],
    )
    .await
    .unwrap();
    assert!(result.ok, "{result:?}");
    let data = result.data.unwrap();
    let sections = data["sections"].as_array().unwrap();
    assert!(sections.len() >= 4, "指令/场景/钉选/检索都应有段落");
}

#[test]
fn editor_tick_reports_idle_state() {
    let app = mock_app_with_kernel();
    let state = || app.state::<AppState>();
    let result = editor_tick(
        state(),
        EditorTickInput {
            project_id: "p".into(),
            chapter_id: "c".into(),
            revision: 1,
            chars_since_commit: 5, // 少于 20 字阈值
            composing: false,
            focused: true,
        },
    );
    assert!(result.ok);
    assert_eq!(result.data.unwrap()["shouldEmitIdle"], false);
}

#[tokio::test]
async fn install_plugin_manifest_validates() {
    let app = mock_app_with_kernel();
    let state = || app.state::<AppState>();

    let manifest = json!({
        "id": "continuity-checker",
        "name": "连续性检查",
        "version": "0.1.0",
        "apiVersion": 1,
        "platforms": ["linux"],
        "operations": [],
        "requestedCapabilities": [{ "kind": "log" }],
    })
    .to_string();

    let ok = install_plugin_manifest(state(), manifest).await.unwrap();
    assert!(ok.ok, "{ok:?}");

    let bad = install_plugin_manifest(state(), "{ 不是清单 }".into())
        .await
        .unwrap();
    assert!(!bad.ok);
}

#[test]
fn kernel_tools_describes_registry() {
    let app = mock_app_with_kernel();
    let state = || app.state::<AppState>();
    let result = kernel_tools(state());
    assert!(result.ok);
    let data = result.data.unwrap();
    let tools: Vec<&str> = data["tools"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|t| t["id"].as_str())
        .collect();
    for expected in [
        "queue.tick",
        "context.hints",
        "document.save",
        "plugin.install",
    ] {
        assert!(tools.contains(&expected), "缺少工具 {expected}: {tools:?}");
    }
    let providers = data["providers"].as_array().unwrap();
    assert!(providers.iter().any(|p| p == &json!("echo")));
}
