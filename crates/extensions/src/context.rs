//! 上下文装配扩展：`context.assemble` 工具按三级预算装配上下文包。

use async_trait::async_trait;
use novel_context_engine::{assemble_context, AssemblyOptions};
use novel_domain::{ChapterId, ProjectId, Revision, WorkContextRef};
use novel_kernel::{Extension, KernelBuilder, KernelError, Tool, ToolContext};
use serde::Deserialize;
use serde_json::{json, Value};

pub struct ContextAssembleTool;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AssembleInput {
    project_id: String,
    chapter_id: String,
    revision: u64,
    instruction: String,
    current_scene: String,
    #[serde(default)]
    pinned: Vec<String>,
    #[serde(default)]
    retrieved: Vec<String>,
    #[serde(default)]
    summaries: Vec<String>,
    #[serde(default = "default_budget")]
    token_budget: u32,
}

fn default_budget() -> u32 {
    12_000
}

#[async_trait]
impl Tool for ContextAssembleTool {
    fn id(&self) -> &str {
        "context.assemble"
    }

    fn summary(&self) -> &str {
        "按优先级与 token 预算装配上下文包"
    }

    async fn execute(&self, input: Value, _ctx: &ToolContext<'_>) -> Result<Value, KernelError> {
        let parsed: AssembleInput =
            serde_json::from_value(input).map_err(|error| KernelError::ToolFailed {
                tool: "context.assemble".into(),
                message: format!("无效的装配参数: {error}"),
            })?;

        let Ok(project_id) = parsed.project_id.parse::<ProjectId>() else {
            return Err(KernelError::ToolFailed {
                tool: "context.assemble".into(),
                message: format!("invalid project id: {}", parsed.project_id),
            });
        };

        let package = assemble_context(
            WorkContextRef {
                project_id,
                branch_id: "main".into(),
                revision: Revision(parsed.revision),
                chapter_id: parsed
                    .chapter_id
                    .parse::<ChapterId>()
                    .unwrap_or_else(|_| ChapterId::new()),
                block_id: None,
                pov_entity_id: None,
            },
            &parsed.instruction,
            &parsed.current_scene,
            &parsed.pinned,
            &parsed.retrieved,
            &parsed.summaries,
            AssemblyOptions {
                token_budget: parsed.token_budget,
            },
        );
        Ok(json!(package))
    }
}

/// 上下文装配扩展。
pub struct ContextAssemblyExtension;

impl Extension for ContextAssemblyExtension {
    fn id(&self) -> &str {
        "builtin.context-assembly"
    }

    fn setup(&self, builder: &mut KernelBuilder) -> Result<(), KernelError> {
        builder.register_tool(ContextAssembleTool);
        Ok(())
    }
}
