//! 上下文浮带扩展：`context.hints` 工具从正史存储读取实体/事实/伏笔，
//! 用 HintEngine 实时匹配。

use crate::util::with_repository;
use async_trait::async_trait;
use novel_context_hints::{HintEngine, HintQuery};
use novel_domain::{ChapterId, ProjectId, Revision, WorkContextRef};
use novel_kernel::{Extension, KernelBuilder, KernelError, Tool, ToolContext};
use serde::Deserialize;
use serde_json::{json, Value};

pub struct ContextHintsTool;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HintsInput {
    project_id: String,
    chapter_id: String,
    revision: u64,
    nearby_text: String,
    generation: u64,
    #[serde(default = "default_limit")]
    limit: usize,
}

fn default_limit() -> usize {
    5
}

#[async_trait]
impl Tool for ContextHintsTool {
    fn id(&self) -> &str {
        "context.hints"
    }

    fn summary(&self) -> &str {
        "实时匹配上下文提示（人物状态、世界规则、开放伏笔）"
    }

    async fn execute(&self, input: Value, ctx: &ToolContext<'_>) -> Result<Value, KernelError> {
        let parsed: HintsInput =
            serde_json::from_value(input).map_err(|error| KernelError::ToolFailed {
                tool: "context.hints".into(),
                message: format!("无效的查询参数: {error}"),
            })?;

        let Ok(project_id) = parsed.project_id.parse::<ProjectId>() else {
            return Err(KernelError::ToolFailed {
                tool: "context.hints".into(),
                message: format!("invalid project id: {}", parsed.project_id),
            });
        };
        let chapter_id = parsed.chapter_id.parse::<ChapterId>().unwrap_or_default();

        let (entities, facts, threads) = with_repository(ctx.kernel(), |repository| {
            Ok((
                repository.list_canon_entities()?,
                repository.list_canon_facts()?,
                repository.list_plot_threads()?,
            ))
        })?;

        let query = HintQuery {
            work_ref: WorkContextRef {
                project_id,
                branch_id: "main".into(),
                revision: Revision(parsed.revision),
                chapter_id,
                block_id: None,
                pov_entity_id: None,
            },
            nearby_text: parsed.nearby_text,
            generation: parsed.generation,
            limit: parsed.limit.clamp(1, 6),
        };
        let hints = HintEngine {
            minimum_dwell_score: 0.2,
        }
        .rank(&query, &entities, &facts, &threads);
        Ok(json!(hints))
    }
}

/// 上下文浮带扩展。
pub struct HintsExtension;

impl Extension for HintsExtension {
    fn id(&self) -> &str {
        "builtin.hints"
    }

    fn setup(&self, builder: &mut KernelBuilder) -> Result<(), KernelError> {
        builder.register_tool(ContextHintsTool);
        Ok(())
    }
}
