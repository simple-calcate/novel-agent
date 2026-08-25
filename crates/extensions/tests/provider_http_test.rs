//! OpenAI 兼容 Provider 的真实 HTTP 集成测试：经 wiremock 模拟服务端，
//! 覆盖流式输出、usage 统计、鉴权头与非 200 错误映射，整条链路走
//! `Kernel::run_continuation`。

use novel_domain::{ChapterId, ProjectId, Revision};
use novel_extensions::ProvidersExtension;
use novel_kernel::{AgentBudget, AgentSpec, Kernel, KernelError, ProviderConfig};
use serde_json::json;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const SSE_BODY: &str = "\
data: {\"choices\":[{\"delta\":{\"content\":\"雾在\"}}]}\n\
data: {\"choices\":[{\"delta\":{\"content\":\"潮响\"}}]}\n\
data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}\n\
data: {\"choices\":[],\"usage\":{\"prompt_tokens\":11,\"completion_tokens\":7}}\n\
data: [DONE]\n\n";

fn spec() -> AgentSpec {
    AgentSpec {
        id: Default::default(),
        project_id: ProjectId::new(),
        chapter_id: ChapterId::new(),
        base_revision: Revision(0),
        prompt: "续写".into(),
        context_text: "雾港".into(),
        budget: AgentBudget {
            max_tokens: 1024,
            max_seconds: 30,
            ..Default::default()
        },
        system_prompt: None,
        temperature: 0.8,
        emit_finish_event: false,
    }
}

fn kernel() -> Kernel {
    Kernel::builder()
        .extension(ProvidersExtension)
        .unwrap()
        .build()
        .unwrap()
}

fn text_of(patch: &novel_domain::ContentPatch) -> String {
    match &patch.operations[0] {
        novel_domain::TextOperation::Insert { text, .. } => text.clone(),
        _ => unreachable!(),
    }
}

#[tokio::test]
async fn streams_sse_response_with_usage() {
    let server = MockServer::start().await;
    // Authorization 匹配不上会得到 404，从而间接验证鉴权头确实发送
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(header("Authorization", "Bearer test-key"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_bytes(SSE_BODY.as_bytes().to_vec()),
        )
        .mount(&server)
        .await;

    let config = ProviderConfig {
        provider: "deepseek".into(),
        api_key: "test-key".into(),
        base_url: server.uri(),
        model: "deepseek-chat".into(),
    };
    let report = kernel().run_continuation(&config, spec()).await.unwrap();

    assert!(!report.truncated);
    assert_eq!(text_of(&report.patch), "雾在潮响");
    assert_eq!(report.input_tokens, 11);
    assert_eq!(report.output_tokens, 7);
    assert_eq!(
        report.patch.created_by,
        novel_domain::Actor::Agent {
            model: "deepseek".into()
        }
    );
}

#[tokio::test]
async fn http_error_maps_to_provider_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(401).set_body_json(
            json!({"error": {"message": "Invalid API key", "code": "invalid_request_error"}}),
        ))
        .mount(&server)
        .await;

    let config = ProviderConfig {
        provider: "openai".into(),
        api_key: "bad-key".into(),
        base_url: server.uri(),
        model: "gpt-test".into(),
    };
    let error = kernel()
        .run_continuation(&config, spec())
        .await
        .expect_err("应返回错误");
    let message = error.to_string();
    assert!(
        message.contains("Invalid API key"),
        "错误应透传服务端信息: {message}"
    );
    assert!(matches!(error, KernelError::Provider(_)));
}

#[tokio::test]
async fn missing_config_is_rejected_before_http() {
    let config = ProviderConfig {
        provider: "deepseek".into(),
        api_key: String::new(), // 缺 key
        base_url: "http://127.0.0.1:9".into(),
        model: "m".into(),
    };
    let error = kernel()
        .run_continuation(&config, spec())
        .await
        .expect_err("应拒绝空 key");
    assert!(error.to_string().contains("api key"));
}

#[tokio::test]
async fn echo_factory_is_registered() {
    let report = kernel()
        .run_continuation(
            &ProviderConfig {
                provider: "echo".into(),
                ..Default::default()
            },
            spec(),
        )
        .await
        .unwrap();
    assert!(text_of(&report.patch).contains("请配置真实模型提供方"));
}
