//! 上下文浮带扩展：`context.hints` 用当前段落匹配预先设计的人物 / 设定 / 伏笔。

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
    #[serde(default)]
    lookback_text: String,
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
        "按当前段落匹配预先设计的人物、设定和伏笔"
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

        let entries = with_repository(ctx.kernel(), |repository| {
            repository.list_story_entries(&project_id)
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
            lookback_text: parsed.lookback_text,
            generation: parsed.generation,
            limit: parsed.limit.clamp(1, 6),
        };
        let hints = HintEngine {
            minimum_dwell_score: 0.2,
        }
        .rank_entries(&query, &entries);
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
