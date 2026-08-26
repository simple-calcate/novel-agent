//! 内置核心工具：文档保存、索引重建、连续性检查、备份创建、
//! Agent 续写/执行。全部注册进内核工具注册表，同名注册可覆盖。

use crate::util::with_repository;
use async_trait::async_trait;
use novel_domain::StoryInstant;
use novel_domain::{FactStatus, ProjectId, Revision};
use novel_kernel::{
    AgentSpec, Extension, KernelBuilder, KernelError, ProviderConfig, Tool, ToolContext,
};
use novel_story_model::{validate_at, ContinuityIssue};
use serde::Deserialize;
use serde_json::{json, Value};

fn string_field(input: &Value, key: &str) -> Option<String> {
    input.get(key).and_then(Value::as_str).map(str::to_owned)
}

fn project_id(input: &Value) -> Result<ProjectId, KernelError> {
    string_field(input, "projectId")
        .and_then(|value| value.parse().ok())
        .ok_or_else(|| KernelError::ToolFailed {
            tool: "core".into(),
            message: "payload 缺少合法的 projectId".into(),
        })
}

pub struct DocumentSaveTool;

#[async_trait]
impl Tool for DocumentSaveTool {
    fn id(&self) -> &str {
        "document.save"
    }

    fn summary(&self) -> &str {
        "记录一次文档保存（正文由编辑器事务提交，这里登记保存事件）"
    }

    async fn execute(&self, input: Value, _ctx: &ToolContext<'_>) -> Result<Value, KernelError> {
        Ok(json!({
            "saved": true,
            "projectId": input.get("projectId").cloned().unwrap_or(Value::Null),
            "savedAt": chrono::Utc::now().to_rfc3339(),
        }))
    }
}

pub struct RebuildIndexTool;

#[async_trait]
impl Tool for RebuildIndexTool {
    fn id(&self) -> &str {
        "index.rebuild"
    }

    fn summary(&self) -> &str {
        "用正史实体重建 FTS 检索索引"
    }

    async fn execute(&self, input: Value, ctx: &ToolContext<'_>) -> Result<Value, KernelError> {
        let project_id = project_id(&input)?;
        let indexed = with_repository(ctx.kernel(), |repository| {
            repository.rebuild_search_index(&project_id)
        })?;
        Ok(json!({"indexed": indexed}))
    }
}

pub struct ContinuityCheckTool;

#[async_trait]
impl Tool for ContinuityCheckTool {
    fn id(&self) -> &str {
        "continuity.check"
    }

    fn summary(&self) -> &str {
        "基于正史模型检查连续性问题"
    }

    async fn execute(&self, input: Value, ctx: &ToolContext<'_>) -> Result<Value, KernelError> {
        let project_id = string_field(&input, "projectId").and_then(|value| value.parse().ok());
        let (issues, open_threads) = with_repository(ctx.kernel(), |repository| {
            let facts = match &project_id {
                Some(id) => {
                    repository.list_canon_facts_for_project(id, Some(FactStatus::Accepted))?
                }
                None => repository
                    .list_canon_facts()?
                    .into_iter()
                    .filter(|fact| fact.status == FactStatus::Accepted)
                    .collect(),
            };
            let threads = match &project_id {
                Some(id) => repository.list_plot_threads_for_project(id)?,
                None => repository.list_plot_threads()?,
            };
            let issues = validate_at(
                &facts,
                &[],
                &[],
                &StoryInstant {
                    sequence: i64::MAX,
                    label: None,
                },
            );
            Ok((issues, threads))
        })?;
        let mut issues: Vec<ContinuityIssue> = issues;
        let open = open_threads
            .iter()
            .filter(|thread| thread.status == novel_domain::PlotThreadStatus::Open)
            .count();
        if open > 0 {
            issues.push(ContinuityIssue {
                severity: novel_story_model::IssueSeverity::Info,
                code: "open-foreshadowing".into(),
                message: format!("仍有 {open} 条未兑现伏笔"),
                evidence: open_threads
                    .iter()
                    .filter(|thread| thread.status == novel_domain::PlotThreadStatus::Open)
                    .map(|thread| thread.title.clone())
                    .collect(),
            });
        }
        Ok(json!({ "issues": issues }))
    }
}

pub struct CreateBackupTool;

#[async_trait]
impl Tool for CreateBackupTool {
    fn id(&self) -> &str {
        "backup.create"
    }

    fn summary(&self) -> &str {
        "把当前快照内容写入 result_objects 备份表"
    }

    async fn execute(&self, input: Value, ctx: &ToolContext<'_>) -> Result<Value, KernelError> {
        let project_id = project_id(&input)?;
        let backup_id = with_repository(ctx.kernel(), |repository| {
            repository.save_result_object(&project_id, "application/json", &input.to_string())
        })?;
        Ok(json!({"backup": true, "backupId": backup_id.to_string()}))
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentJobInput {
    #[serde(default)]
    chapter_id: Option<String>,
    #[serde(default)]
    revision: Option<u64>,
    #[serde(default)]
    prompt: Option<String>,
    #[serde(default)]
    context_text: Option<String>,
    #[serde(default)]
    max_tokens: Option<u32>,
    #[serde(default)]
    system_prompt: Option<String>,
}

/// Agent 续写工具：从应用配置读取模型设置，走内核预算约束的续写循环。
pub struct AgentContinuationTool;

#[async_trait]
impl Tool for AgentContinuationTool {
    fn id(&self) -> &str {
        "agent.continuation"
    }

    fn summary(&self) -> &str {
        "按章节上下文生成 AI 续写候选"
    }

    async fn execute(&self, input: Value, ctx: &ToolContext<'_>) -> Result<Value, KernelError> {
        run_agent_tool("agent.continuation", input, ctx).await
    }
}

/// Agent 通用执行工具（prompt 驱动）。
pub struct AgentRunTool;

#[async_trait]
impl Tool for AgentRunTool {
    fn id(&self) -> &str {
        "agent.run"
    }

    fn summary(&self) -> &str {
        "按给定的提示词执行一次 Agent 任务"
    }

    async fn execute(&self, input: Value, ctx: &ToolContext<'_>) -> Result<Value, KernelError> {
        run_agent_tool("agent.run", input, ctx).await
    }
}

async fn run_agent_tool(
    tool_id: &str,
    input: Value,
    ctx: &ToolContext<'_>,
) -> Result<Value, KernelError> {
    let job: AgentJobInput =
        serde_json::from_value(input.clone()).map_err(|error| KernelError::ToolFailed {
            tool: tool_id.to_owned(),
            message: format!("解析任务载荷失败: {error}"),
        })?;

    let config = load_provider_config(ctx)?;
    let chapter_id = job
        .chapter_id
        .and_then(|value| value.parse().ok())
        .unwrap_or_default();
    let revision = Revision(job.revision.unwrap_or(0));
    let prompt = job.prompt.unwrap_or_else(|| "继续当前剧情".into());

    let spec = AgentSpec {
        id: Default::default(),
        project_id: project_id(&input)?,
        chapter_id,
        base_revision: revision,
        prompt,
        context_text: job.context_text.unwrap_or_default(),
        budget: novel_kernel::AgentBudget {
            max_tokens: job.max_tokens.unwrap_or(2048),
            ..Default::default()
        },
        system_prompt: job.system_prompt,
        temperature: 0.8,
        emit_finish_event: false,
    };

    let report = ctx.kernel().run_continuation(&config, spec).await?;
    Ok(json!({
        "patch": report.patch,
        "truncated": report.truncated,
        "outputTokens": report.output_tokens,
    }))
}

/// 从 app_settings.model_config 读取模型配置；缺失时回退 echo。
pub fn load_provider_config(ctx: &ToolContext<'_>) -> Result<ProviderConfig, KernelError> {
    crate::workspace::load_provider_config_from_kernel(ctx.kernel())
}

/// 核心工具扩展。
pub struct CoreToolsExtension;

impl Extension for CoreToolsExtension {
    fn id(&self) -> &str {
        "builtin.core-tools"
    }

    fn setup(&self, builder: &mut KernelBuilder) -> Result<(), KernelError> {
        builder.register_tool(DocumentSaveTool);
        builder.register_tool(RebuildIndexTool);
        builder.register_tool(ContinuityCheckTool);
        builder.register_tool(CreateBackupTool);
        builder.register_tool(AgentContinuationTool);
        builder.register_tool(AgentRunTool);
        Ok(())
    }
}
