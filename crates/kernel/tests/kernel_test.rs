use async_trait::async_trait;
use futures::stream::BoxStream;
use futures::StreamExt;
use novel_domain::{
    Actor, ChapterId, DomainEvent, EventId, EventSource, Platform, ProjectId, Revision,
};
use novel_kernel::{
    AgentBudget, AgentSpec, EventSubscriber, Extension, Kernel, KernelBuilder, KernelError,
    ModelChunk, ModelError, ModelProvider, ModelRequest, ProviderConfig, ProviderFactory,
};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// 可编排的假 Provider：chunks=0 表示无限流。
#[derive(Clone)]
struct ScriptedProvider {
    chunks: usize,
    chunk_chars: usize,
    delay_ms: u64,
}

#[async_trait]
impl ModelProvider for ScriptedProvider {
    fn name(&self) -> &str {
        "scripted"
    }

    async fn stream(
        &self,
        _request: ModelRequest,
    ) -> Result<BoxStream<'static, Result<ModelChunk, ModelError>>, ModelError> {
        let total = self.chunks;
        let text = "字".repeat(self.chunk_chars);

        if total == 0 {
            // 无限流：每 delay 毫秒产出一个分块，done 永远为 false
            let delay = std::time::Duration::from_millis(self.delay_ms);
            let stream = futures::stream::unfold(0usize, move |i| {
                let text = text.clone();
                async move {
                    tokio::time::sleep(delay).await;
                    Some((scripted_chunk(&text, i, total), i + 1))
                }
            });
            return Ok(Box::pin(stream));
        }

        if self.delay_ms == 0 {
            let stream =
                futures::stream::iter((0..total).map(move |i| scripted_chunk(&text, i, total)));
            return Ok(Box::pin(stream));
        }

        let delay = std::time::Duration::from_millis(self.delay_ms);
        let stream = futures::stream::iter(
            (0..total).map(move |i| scripted_chunk(&text, i, total)),
        )
        .then(move |chunk| async move {
            tokio::time::sleep(delay).await;
            chunk
        });
        Ok(Box::pin(stream))
    }
}

fn scripted_chunk(text: &str, index: usize, total: usize) -> Result<ModelChunk, ModelError> {
    Ok(ModelChunk {
        text: text.to_owned(),
        input_tokens: None,
        output_tokens: None,
        done: total != 0 && index + 1 == total,
    })
}

#[derive(Clone)]
struct ScriptedFactory {
    provider: ScriptedProvider,
}

impl ProviderFactory for ScriptedFactory {
    fn create(&self, _config: &ProviderConfig) -> Result<Arc<dyn ModelProvider>, KernelError> {
        Ok(Arc::new(self.provider.clone()))
    }
}

fn spec(budget: AgentBudget, emit_event: bool) -> AgentSpec {
    AgentSpec {
        id: Default::default(),
        project_id: ProjectId::new(),
        chapter_id: ChapterId::new(),
        base_revision: Revision(3),
        prompt: "续写这一段".into(),
        context_text: "雾港的夜晚".into(),
        budget,
        system_prompt: None,
        temperature: 0.8,
        emit_finish_event: emit_event,
    }
}

fn kernel_with(provider: ScriptedProvider) -> Kernel {
    Kernel::builder()
        .provider_factory("scripted", ScriptedFactory { provider })
        .build()
        .unwrap()
}

fn config() -> ProviderConfig {
    ProviderConfig {
        provider: "scripted".into(),
        model: "test-model".into(),
        ..Default::default()
    }
}

#[tokio::test]
async fn completes_when_provider_finishes() {
    // 3 块 × 100 字（约 50 token/块）
    let kernel = kernel_with(ScriptedProvider {
        chunks: 3,
        chunk_chars: 100,
        delay_ms: 0,
    });
    let report = kernel
        .run_continuation(
            &config(),
            spec(
                AgentBudget {
                    max_tokens: 500,
                    max_seconds: 30,
                    ..Default::default()
                },
                false,
            ),
        )
        .await
        .unwrap();
    assert!(!report.truncated);
    assert_eq!(report.patch.operations.len(), 1);
    assert_eq!(report.output_tokens, 150); // 300 字 / 2
    assert_eq!(report.patch.base_revision, Revision(3));
}

#[tokio::test]
async fn token_budget_truncates_stream() {
    // 上限 120 token：第 3 块（累计 150）后截断
    let kernel = kernel_with(ScriptedProvider {
        chunks: 10,
        chunk_chars: 100,
        delay_ms: 0,
    });
    let report = kernel
        .run_continuation(
            &config(),
            spec(
                AgentBudget {
                    max_tokens: 120,
                    max_seconds: 30,
                    ..Default::default()
                },
                false,
            ),
        )
        .await
        .unwrap();
    assert!(report.truncated);
    assert_eq!(report.output_tokens, 150);
    let text = match &report.patch.operations[0] {
        novel_domain::TextOperation::Insert { text, .. } => text.clone(),
        _ => unreachable!(),
    };
    assert_eq!(text.chars().count(), 300);
}

#[tokio::test]
async fn time_budget_truncates_stream() {
    let kernel = kernel_with(ScriptedProvider {
        chunks: 0, // 无限流
        chunk_chars: 10,
        delay_ms: 20,
    });
    let started = std::time::Instant::now();
    let report = kernel
        .run_continuation(
            &config(),
            spec(
                AgentBudget {
                    max_tokens: 100_000,
                    max_seconds: 1,
                    ..Default::default()
                },
                false,
            ),
        )
        .await
        .unwrap();
    assert!(report.truncated);
    assert!(started.elapsed() >= std::time::Duration::from_secs(1));
    assert!(started.elapsed() < std::time::Duration::from_secs(3));
}

#[tokio::test]
async fn unknown_provider_is_rejected() {
    let kernel = Kernel::builder().build().unwrap();
    let result = kernel
        .run_continuation(
            &ProviderConfig {
                provider: "nope".into(),
                ..Default::default()
            },
            spec(AgentBudget::default(), false),
        )
        .await;
    assert!(matches!(
        result,
        Err(KernelError::ProviderNotFound(name)) if name == "nope"
    ));
}

#[derive(Default)]
struct Recorder {
    seen: Mutex<Vec<String>>,
}

struct RecorderSubscriber {
    recorder: Arc<Recorder>,
    types: &'static [&'static str],
}

impl EventSubscriber for RecorderSubscriber {
    fn id(&self) -> &str {
        "recorder"
    }

    fn event_types(&self) -> &[&str] {
        self.types
    }

    fn handle(&self, _kernel: &Kernel, event: &DomainEvent) -> Result<Value, KernelError> {
        self.recorder
            .seen
            .lock()
            .unwrap()
            .push(event.event_type.clone());
        Ok(json!({"ok": 1}))
    }
}

/// 把记录器注册进内核的扩展（验证扩展可以携带状态注册订阅者）。
struct RecorderExtension {
    subscriber: RecorderSubscriber,
}

impl Extension for RecorderExtension {
    fn id(&self) -> &str {
        "test.recorder"
    }

    fn setup(&self, builder: &mut KernelBuilder) -> Result<(), KernelError> {
        let subscriber = RecorderSubscriber {
            recorder: self.subscriber.recorder.clone(),
            types: self.subscriber.types,
        };
        builder.add_subscriber(subscriber);
        Ok(())
    }
}

fn event(event_type: &str) -> DomainEvent {
    DomainEvent {
        event_id: EventId::new(),
        event_type: event_type.into(),
        schema_version: 1,
        occurred_at: chrono::Utc::now(),
        project_id: ProjectId::new(),
        book_id: None,
        chapter_id: None,
        scene_id: None,
        block_id: None,
        actor: Actor::User { user_id: None },
        source: EventSource::Editor,
        platform: Platform::Linux,
        transaction_id: Uuid::new_v4().to_string(),
        correlation_id: None,
        causation_id: None,
        revision_before: Revision(0),
        revision_after: Revision(1),
        payload: json!({}),
    }
}

#[tokio::test]
async fn dispatch_routes_by_event_type() {
    let recorder = Arc::new(Recorder::default());
    let kernel = Kernel::builder()
        .extension(RecorderExtension {
            subscriber: RecorderSubscriber {
                recorder: recorder.clone(),
                types: &["editor.idle"],
            },
        })
        .unwrap()
        .build()
        .unwrap();

    kernel.dispatch(&event("editor.idle"));
    kernel.dispatch(&event("chapter.created"));
    assert_eq!(*recorder.seen.lock().unwrap(), vec!["editor.idle"]);

    // 空切片 = 订阅全部
    let recorder_all = Arc::new(Recorder::default());
    let kernel = Kernel::builder()
        .extension(RecorderExtension {
            subscriber: RecorderSubscriber {
                recorder: recorder_all.clone(),
                types: &[],
            },
        })
        .unwrap()
        .build()
        .unwrap();
    kernel.dispatch(&event("editor.idle"));
    kernel.dispatch(&event("chapter.created"));
    assert_eq!(recorder_all.seen.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn continuation_publishes_finish_event() {
    let recorder = Arc::new(Recorder::default());
    let kernel = Kernel::builder()
        .provider_factory(
            "scripted",
            ScriptedFactory {
                provider: ScriptedProvider {
                    chunks: 1,
                    chunk_chars: 10,
                    delay_ms: 0,
                },
            },
        )
        .extension(RecorderExtension {
            subscriber: RecorderSubscriber {
                recorder: recorder.clone(),
                types: &[],
            },
        })
        .unwrap()
        .build()
        .unwrap();

    kernel
        .run_continuation(&config(), spec(AgentBudget::default(), true))
        .await
        .unwrap();
    assert_eq!(*recorder.seen.lock().unwrap(), vec!["agent.finished"]);
}

#[tokio::test]
async fn subscriber_failure_does_not_block_others() {
    use novel_kernel::SubscriberOutcome;

    struct FailingSubscriber;

    impl EventSubscriber for FailingSubscriber {
        fn id(&self) -> &str {
            "failing"
        }

        fn handle(&self, _kernel: &Kernel, _event: &DomainEvent) -> Result<Value, KernelError> {
            Err(KernelError::Storage("订阅者内部错误".into()))
        }
    }

    let recorder = Arc::new(Recorder::default());
    let mut builder = Kernel::builder();
    builder.add_subscriber(FailingSubscriber);
    let kernel = builder
        .extension(RecorderExtension {
            subscriber: RecorderSubscriber {
                recorder: recorder.clone(),
                types: &[],
            },
        })
        .unwrap()
        .build()
        .unwrap();

    let summary = kernel.dispatch(&event("editor.idle"));

    // 失败被记录在结果里，但不阻断后续订阅者
    assert!(summary.has_error());
    assert_eq!(summary.first_error(), Some("storage error: 订阅者内部错误"));
    assert_eq!(*recorder.seen.lock().unwrap(), vec!["editor.idle"]);
    let outcomes: Vec<&SubscriberOutcome> = summary.outcomes.iter().collect();
    assert_eq!(outcomes.len(), 2, "两个订阅者都应被调用");
}
